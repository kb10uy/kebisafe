//! Contains application common types.

use async_std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};

use aes_gcm_siv::{
    aead::{generic_array::GenericArray, NewAead},
    Aes256GcmSiv,
};
use anyhow::Result;
use clap::Clap;
use data_encoding::HEXLOWER_PERMISSIVE;
use serde::Deserialize;

/// Captured environment variables.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Environments {
    pub listen_at: SocketAddr,
    pub secret_key: String,
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
    /// Root directory of static file serving
    pub public_root: PathBuf,

    /// Cipher
    pub cipher: Aes256GcmSiv,

    /// Account's name and password hash.
    pub account: (String, String),
}

impl State {
    /// Constructs new application state.
    pub fn new(envs: &Environments, public_path: impl AsRef<Path>) -> Result<(Arc<State>, Box<[u8]>)> {
        let secret_key = HEXLOWER_PERMISSIVE.decode(envs.secret_key.as_bytes())?.into_boxed_slice();
        let public_root = public_path.as_ref().into();

        let key_array = GenericArray::from_slice(&secret_key);
        let cipher = Aes256GcmSiv::new(key_array);

        Ok((
            Arc::new(State {
                public_root,
                cipher,
                account: (envs.account_name.clone(), envs.account_password.clone()),
            }),
            secret_key,
        ))
    }
}
