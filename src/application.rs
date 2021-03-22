//! Contains application common types.

use async_std::{net::SocketAddr, path::Path};

use anyhow::Result;
use async_session::MemoryStore;
use clap::Clap;
use serde::Deserialize;

/// Captured environment variables.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Environments {
    pub listen_at: SocketAddr,
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
#[derive(Debug, Clone)]
pub struct State {
    /// HTTP session store
    pub session_store: MemoryStore,

    /// Root directory of static file serving
    pub public_root: Box<Path>,

    /// Account's name and password hash.
    pub account: (String, String),
}

impl State {
    /// Constructs new application state.
    pub fn new(envs: &Environments, public_path: impl AsRef<Path>) -> Result<State> {
        let session_store = MemoryStore::new();
        let public_root = public_path.as_ref().into();

        Ok(State {
            session_store,
            public_root,
            account: (envs.account_name.clone(), envs.account_password.clone()),
        })
    }
}
