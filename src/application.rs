//! Contains application common types.

use async_std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use std::{
    convert::TryFrom,
    fmt::{Debug, Formatter, Result as FmtResult},
};

use aes_gcm_siv::{
    aead::{generic_array::GenericArray, NewAead},
    Aes256GcmSiv,
};
use anyhow::Result;
use async_trait::async_trait;
use clap::Clap;
use data_encoding::HEXLOWER_PERMISSIVE;
use redis::{aio::Connection as RedisConnection, AsyncCommands, Client as RedisClient};
use serde::Deserialize;
use sqlx::PgPool;
use tide::sessions::{Session, SessionStore};
use url::Url;

/// Captured environment variables.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Environments {
    pub secret_key: String,
    pub listen_at: String,
    pub hosted_at: String,
    pub database_uri: String,
    pub redis_uri: String,
    pub public_dir: String,
    pub media_dir: String,
    pub account_name: String,
    pub account_password: String,
}

/// Commandline arguments.
#[derive(Debug, Clap)]
pub struct Arguments {
    /// Executing subcommand (default to `serve`)
    #[clap(subcommand)]
    pub subcommand: Option<Subcommand>,
}

#[derive(Debug, Clap)]
pub enum Subcommand {
    /// Starts Kebisafe server
    Serve,

    /// Generates password hash
    GeneratePassword,
}

/// Shared application state for the server.
#[derive(Clone)]
pub struct State {
    /// Local media root
    pub media_root: PathBuf,

    /// Root URL at which hosted
    pub hosted_at: Url,

    /// Cipher
    pub cipher: Aes256GcmSiv,

    /// Database connection pool
    pub pool: PgPool,

    /// Account's name and password hash.
    pub account: (String, String),
}

impl State {
    /// Constructs new application state.
    pub async fn new(envs: &Environments) -> Result<(Arc<State>, Box<[u8]>)> {
        let media_root = PathBuf::try_from(&envs.media_dir)?;
        let hosted_at = Url::parse(&envs.hosted_at)?;
        let secret_key = HEXLOWER_PERMISSIVE.decode(envs.secret_key.as_bytes())?.into_boxed_slice();
        let key_array = GenericArray::from_slice(&secret_key);
        let cipher = Aes256GcmSiv::new(key_array);
        let pool = PgPool::connect(&envs.database_uri).await?;

        Ok((
            Arc::new(State {
                media_root,
                hosted_at,
                cipher,
                pool,
                account: (envs.account_name.clone(), envs.account_password.clone()),
            }),
            secret_key,
        ))
    }
}

/// Redis backend for session store.
#[derive(Clone)]
pub struct RedisStore {
    client: RedisClient,

    // TODO: Multiplex connection
    connection: Arc<Mutex<RedisConnection>>,

    id_header: String,
}

impl Debug for RedisStore {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "RedisStore {{ internal hidden }}")
    }
}

impl RedisStore {
    /// Connects to the Redis and creates a store.
    pub async fn new(uri: &str) -> Result<RedisStore> {
        let client = RedisClient::open(uri)?;
        let connection = Arc::new(Mutex::new(client.get_async_std_connection().await?));
        let id_header = "kebisafe.session:".into();

        Ok(RedisStore {
            client,
            connection,
            id_header,
        })
    }
}

#[async_trait]
impl SessionStore for RedisStore {
    async fn load_session(&self, cookie_value: String) -> Result<Option<Session>> {
        let mut conn = self.connection.lock().await;
        let mut key = self.id_header.clone();
        key.push_str(&Session::id_from_cookie_value(&cookie_value)?);

        let session_json: Option<String> = conn.get(&key).await?;
        match session_json {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    async fn store_session(&self, session: Session) -> Result<Option<String>> {
        let mut conn = self.connection.lock().await;
        let mut key = self.id_header.clone();
        key.push_str(session.id());

        let session_json = serde_json::to_string(&session)?;
        match session.expires_in() {
            Some(duration) => conn.set_ex(&key, &session_json, duration.as_secs() as usize).await?,
            None => conn.set(&key, &session_json).await?,
        }

        session.reset_data_changed();
        Ok(session.into_cookie_value())
    }

    async fn destroy_session(&self, session: Session) -> Result<()> {
        let mut conn = self.connection.lock().await;
        let mut key = self.id_header.clone();
        key.push_str(session.id());

        conn.del(&key).await?;
        Ok(())
    }

    async fn clear_store(&self) -> Result<()> {
        let mut conn_iter = self.client.get_async_std_connection().await?;
        let mut conn = self.connection.lock().await;
        let mut key_pattern = self.id_header.clone();
        key_pattern.push('*');

        let mut keys = conn_iter.scan_match::<_, String>(&key_pattern).await?;
        while let Some(key) = keys.next_item().await {
            conn.del(&key).await?;
        }
        Ok(())
    }
}
