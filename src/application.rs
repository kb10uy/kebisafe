//! Contains application common types.

use async_std::{net::SocketAddr, sync::Arc};

use aes_gcm_siv::{
    aead::{generic_array::GenericArray, NewAead},
    Aes256GcmSiv,
};
use anyhow::Result;
use clap::Clap;
use data_encoding::HEXLOWER_PERMISSIVE;
use serde::Deserialize;
use sqlx::PgPool;

/// Captured environment variables.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Environments {
    pub listen_at: SocketAddr,
    pub secret_key: String,
    pub database_uri: String,
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
        let secret_key = HEXLOWER_PERMISSIVE.decode(envs.secret_key.as_bytes())?.into_boxed_slice();
        let key_array = GenericArray::from_slice(&secret_key);
        let cipher = Aes256GcmSiv::new(key_array);
        let pool = PgPool::connect(&envs.database_uri).await?;

        Ok((
            Arc::new(State {
                cipher,
                pool,
                account: (envs.account_name.clone(), envs.account_password.clone()),
            }),
            secret_key,
        ))
    }
}
