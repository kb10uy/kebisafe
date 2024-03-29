//! Contains application common types.

use async_std::{path::PathBuf, sync::Arc};
use std::convert::TryFrom;

use aes_gcm_siv::{
    aead::{generic_array::GenericArray, NewAead},
    Aes256GcmSiv,
};
use anyhow::Result;
use clap::{Parser, Subcommand};
use data_encoding::HEXLOWER_PERMISSIVE;
use serde::Deserialize;
use sqlx::PgPool;
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
    pub api_token: String,
}

/// Minimal, single-user, and fast image upload service
#[derive(Debug, Parser)]
#[clap(version, author)]
pub struct Arguments {
    /// Executing subcommand (default to `serve`)
    #[clap(subcommand)]
    pub subcommand: Option<SubCommand>,
}

#[derive(Debug, Subcommand)]
pub enum SubCommand {
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
