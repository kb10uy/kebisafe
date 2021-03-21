//! Contains application common types.

use async_std::net::SocketAddr;

use anyhow::Result;
use serde::Deserialize;
use async_session::MemoryStore;

/// Captured environment variables.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Environments {
    pub listen_at: SocketAddr,
}

/// Shared application state for the server.
#[derive(Debug, Clone)]
pub struct State {
    /// HTTP session store
    pub session_store: MemoryStore,
}

impl State {
    /// Constructs new application state.
    pub fn new() -> Result<State> {
        let session_store = MemoryStore::new();

        Ok(State {
            session_store,
        })
    }
}
