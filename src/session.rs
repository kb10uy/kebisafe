use std::str;

use aes_gcm_siv::{
    aead::{generic_array::GenericArray, Aead},
    Aes256GcmSiv,
};
use anyhow::{ensure, format_err, Result};
use chrono::prelude::*;
use data_encoding::BASE64;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use tide::sessions::Session;

const TOKEN_EXPIARY: i64 = 86400;

/// Represents an account information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Account {
    pub name: String,
}

/// Represents a flash message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Flash {
    Info(String),
    Warning(String),
    Error(String),
}

/// Generates a CSRF token.
pub fn generate_csrf_token(cipher: &Aes256GcmSiv, session: &Session) -> Result<String> {
    let plain_text = format!("{} {}", session.id(), Local::now().timestamp().to_string());

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
    ensure!(sid == token_sid, "Invalid token");

    let now = Local::now().timestamp();
    let token_time = params[1].parse().ok().unwrap_or(0);
    ensure!(now - token_time > TOKEN_EXPIARY, "Expired token");

    Ok(())
}

/// Pops existing flash messages and inserts new ones.
pub fn swap_flashes(session: &mut Session, new_flashes: Vec<Flash>) -> Result<Vec<Flash>> {
    let old_flashes = session.get("flash_messages").unwrap_or_default();
    session.insert("flash_messages", new_flashes)?;

    Ok(old_flashes)
}

#[macro_export]
macro_rules! csrf_protect {
    ($req:expr, $t:expr) => {
        use tide::{http::StatusCode, Response};
        let cipher = &$req.state().cipher;
        let session = $req.session();
        match $crate::session::verify_csrf_token(cipher, session, $t) {
            Ok(()) => (),
            Err(e) => return Ok(Response::builder(StatusCode::BadRequest).body(e.to_string()).build()),
        }
    };
}
