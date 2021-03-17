//! Contains application common types.

use async_std::net::SocketAddr;
use serde::Deserialize;

/// Captured environment variables.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Environments {
    pub listen_at: SocketAddr,
}

/// Shared application state for the server.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct State {}
