//! Sessions

use async_std::sync::{Arc, Mutex};
use std::fmt::{Debug, Formatter, Result as FmtResult};

use anyhow::Result;
use async_trait::async_trait;
use log::{debug, info};
use redis::{aio::Connection as RedisConnection, AsyncCommands, Client as RedisClient};
use tide::sessions::{Session, SessionStore};

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
        debug!("Loading session by Cookie \"{}\"", cookie_value);

        let mut conn = self.connection.lock().await;
        let mut key = self.id_header.clone();
        key.push_str(&Session::id_from_cookie_value(&cookie_value)?);

        let session_json: Option<String> = conn.get(&key).await?;
        let session: Session = match session_json {
            Some(json) => serde_json::from_str(&json)?,
            None => return Ok(None),
        };

        Ok(session.validate())
    }

    async fn store_session(&self, session: Session) -> Result<Option<String>> {
        debug!("Storing session id \"{}\"", session.id());

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
        debug!("Destroying session id \"{}\"", session.id());

        let mut conn = self.connection.lock().await;
        let mut key = self.id_header.clone();
        key.push_str(session.id());

        conn.del(&key).await?;
        Ok(())
    }

    async fn clear_store(&self) -> Result<()> {
        debug!("Clearing session store");

        let mut conn_iter = self.client.get_async_std_connection().await?;
        let mut conn = self.connection.lock().await;
        let mut key_pattern = self.id_header.clone();
        key_pattern.push('*');

        let mut cleared_count = 0usize;
        let mut keys = conn_iter.scan_match::<_, String>(&key_pattern).await?;
        while let Some(key) = keys.next_item().await {
            conn.del(&key).await?;
            cleared_count += 1;
        }

        info!("Cleared {} sessions", cleared_count);
        Ok(())
    }
}
