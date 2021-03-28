//! Sessions

use async_std::sync::{Arc, Mutex};
use std::fmt::{Debug, Formatter, Result as FmtResult};

use anyhow::Result;
use async_trait::async_trait;
use log::{debug, info};
use redis::{aio::Connection as RedisConnection, AsyncCommands, Client as RedisClient};
use tide::sessions::{Session, SessionStore};

/// A workaround for tide issue #762.
/// https://github.com/http-rs/tide/issues/762
pub trait SessionWorkaroundExt {
    /// Session key of regeneration flag.
    const REGENERATION_MARK_KEY: &'static str;

    /// Marks the session for ID regeneration.
    fn mark_for_regenerate(&mut self);

    /// Checks whether the session should regenerate the ID.
    /// The session key `REGENERATION_MARK` will be removved.
    fn should_regenerate(&mut self) -> bool;
}

impl SessionWorkaroundExt for Session {
    const REGENERATION_MARK_KEY: &'static str = "sid-regenerate";

    fn mark_for_regenerate(&mut self) {
        self.insert(Self::REGENERATION_MARK_KEY, true).expect("Boolean should be serialized");
    }

    fn should_regenerate(&mut self) -> bool {
        let previously_changed = self.data_changed();
        let regenerate = self.get(Self::REGENERATION_MARK_KEY).unwrap_or_default();

        self.remove(Self::REGENERATION_MARK_KEY);
        if !previously_changed {
            self.reset_data_changed();
        }

        regenerate
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

    /// Generates Redis key from session.
    pub fn redis_key(&self, original_key: &str) -> String {
        let mut key = self.id_header.clone();
        key.push_str(original_key);
        key
    }
}

#[async_trait]
impl SessionStore for RedisStore {
    async fn load_session(&self, cookie_value: String) -> Result<Option<Session>> {
        debug!("Loading session by Cookie \"{}\"", cookie_value);
        let mut conn = self.connection.lock().await;

        let key = self.redis_key(&Session::id_from_cookie_value(&cookie_value)?);
        let value: Option<String> = conn.get(&key).await?;
        match value {
            Some(json) => {
                let mut session: Session = serde_json::from_str(&json)?;
                session.set_cookie_value(cookie_value);
                Ok(Some(session))
            }
            None => {
                debug!("Session not found: \"{}\"", key);
                Ok(None)
            }
        }
    }

    async fn store_session(&self, mut session: Session) -> Result<Option<String>> {
        debug!("Storing session id \"{}\"", session.id());
        let mut conn = self.connection.lock().await;

        if session.should_regenerate() {
            session.regenerate();
        }

        let key = self.redis_key(session.id());
        let value = serde_json::to_string(&session)?;
        match session.expires_in() {
            Some(duration) => conn.set_ex(&key, &value, duration.as_secs() as usize).await?,
            None => conn.set(&key, &value).await?,
        }

        let cookie_value = session.into_cookie_value();
        debug!("Session stored, Cookie value will be {:?}", cookie_value);
        Ok(cookie_value)
    }

    async fn destroy_session(&self, session: Session) -> Result<()> {
        debug!("Destroying session id \"{}\"", session.id());
        let mut conn = self.connection.lock().await;

        let key = self.redis_key(session.id());
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
