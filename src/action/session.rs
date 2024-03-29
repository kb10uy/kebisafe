//! Contains session manipulation types and functions.

use crate::{application::State, entity::Media};

use std::str;

use aes_gcm_siv::{
    aead::{generic_array::GenericArray, Aead},
    Aes256GcmSiv,
};
use anyhow::{ensure, format_err, Result};
use data_encoding::BASE64;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use tide::sessions::Session;
use time::OffsetDateTime;
use url::Url;

const TOKEN_EXPIARY: i64 = 86400;
const SESSION_ACCOUNT: &'static str = "kebisafe.account";
const SESSION_FLASHES: &'static str = "kebisafe.flashes";

/// Represents an account information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Account {
    pub name: String,
}

/// Represents a flash message.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Flash {
    Info(String),
    Warning(String),
    Error(String),
}

/// Represents common session data.
#[derive(Debug, Clone)]
pub struct Common {
    pub hosted_at: Url,
    pub account: Option<Account>,
    pub flashes: Vec<Flash>,
    pub csrf: String,
}

impl Common {
    /// Constructs with CSRF token.
    pub fn new(state: &State, session: &mut Session, new_flashes: Vec<Flash>) -> Result<Common> {
        let csrf = generate_csrf_token(&state.cipher, session)?;
        Ok(Common {
            hosted_at: state.hosted_at.clone(),
            account: session.get(SESSION_ACCOUNT),
            flashes: swap_flashes(session, new_flashes)?,
            csrf,
        })
    }

    /// Generates original media permalink.
    pub fn permalink_original(&self, media: &Media) -> String {
        let relative = format!("/media/{}.{}", media.hash_id, media.extension);
        self.hosted_at.join(&relative).map(|url| url.to_string()).unwrap_or_default()
    }

    /// Generates thumbnail media permalink.
    pub fn permalink_thumbnail(&self, media: &Media) -> String {
        if media.has_thumbnail {
            let relative = format!("/media/thumbnails/{}.{}", media.hash_id, "jpg");
            self.hosted_at.join(&relative).map(|url| url.to_string()).unwrap_or_default()
        } else {
            self.permalink_original(media)
        }
    }
}

/// Generates a CSRF token.
pub fn generate_csrf_token(cipher: &Aes256GcmSiv, session: &Session) -> Result<String> {
    let plain_text = format!("{} {}", session.id(), OffsetDateTime::now_local()?.unix_timestamp().to_string());

    let nonce = random::<[u8; 12]>();
    let nonce = GenericArray::from_slice(&nonce);
    let mut cipher_bytes = cipher
        .encrypt(&nonce, plain_text.as_bytes())
        .map_err(|_| format_err!("Failed to encrypt token"))?;

    let mut bytes = nonce.to_vec();
    bytes.append(&mut cipher_bytes);

    Ok(BASE64.encode(&bytes))
}

/// Verifies CSRF token.
pub fn verify_csrf_token(cipher: &Aes256GcmSiv, session: &Session, token: &str) -> Result<()> {
    // Decode and decrypt token
    let decoded_buffer = BASE64.decode(token.as_bytes())?;
    ensure!(decoded_buffer.len() >= 12, "Not enough token length");
    let nonce_array = GenericArray::from_slice(&decoded_buffer[..12]);
    let decrypted = cipher
        .decrypt(nonce_array, &decoded_buffer[12..])
        .map_err(|_| format_err!("Failed to decrypt token"))?;
    let params: Vec<_> = str::from_utf8(&decrypted)?.split_whitespace().collect();
    ensure!(params.len() == 2, "Invalid token structure");

    // Verify
    let sid = session.id();
    let token_sid = params[0];
    ensure!(sid == token_sid, "Invalid token; expected \"{}\", found \"{}\"", sid, token_sid);

    let now = OffsetDateTime::now_local()?.unix_timestamp();
    let token_time = params[1].parse().ok().unwrap_or(0);
    ensure!(now - token_time <= TOKEN_EXPIARY, "Expired token");

    Ok(())
}

/// Pops existing flash messages and inserts new ones.
pub fn swap_flashes(session: &mut Session, mut new_flashes: Vec<Flash>) -> Result<Vec<Flash>> {
    let old_flashes = session.get(SESSION_FLASHES).unwrap_or_default();

    new_flashes.sort();
    new_flashes.dedup();
    session.insert(SESSION_FLASHES, new_flashes)?;

    Ok(old_flashes)
}

/// Sets account information into the session.
pub fn set_account(session: &mut Session, account: Account) -> Result<()> {
    session.insert(SESSION_ACCOUNT, account)?;
    Ok(())
}

/// Deletes account information from the session.
pub fn delete_account(session: &mut Session) -> Result<()> {
    session.remove(SESSION_ACCOUNT);
    Ok(())
}

/// Fetches account information from the session.
pub fn get_account(session: &Session) -> Option<Account> {
    session.get(SESSION_ACCOUNT)
}
