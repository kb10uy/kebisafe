use std::{str, time::Duration};

use aes_gcm_siv::{
    aead::{generic_array::GenericArray, Aead, NewAead},
    Aes256GcmSiv,
};
use anyhow::{ensure, format_err, Result};
use async_trait::async_trait;
use chrono::prelude::*;
use data_encoding::BASE64;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tide::{
    http::{mime, Method, StatusCode},
    sessions::Session,
    Middleware, Next, Request, Response, Result as TideResult,
};

const ALLOWED_METHODS: &'static [Method] = &[Method::Get, Method::Head, Method::Options];

/// The middleware which enables CSRF protection.
pub struct CsrfProtectionMiddleware {
    cipher: Aes256GcmSiv,
    token_expiary: u64,
}

impl CsrfProtectionMiddleware {
    pub fn new(key: &[u8], token_expiary: Duration) -> Result<CsrfProtectionMiddleware> {
        ensure!(key.len() == 32, "Key length must be 32 bytes");

        let key_array = GenericArray::from_slice(key);
        let cipher = Aes256GcmSiv::new(key_array);

        Ok(CsrfProtectionMiddleware {
            cipher,
            token_expiary: token_expiary.as_secs(),
        })
    }
}

#[async_trait]
impl<State: Clone + Send + Sync + 'static> Middleware<State> for CsrfProtectionMiddleware {
    async fn handle(&self, mut request: Request<State>, next: Next<'_, State>) -> TideResult {
        if ALLOWED_METHODS.contains(&request.method()) {
            return Ok(next.run(request).await);
        }

        // Retrieve token
        let token = if let Some(header_token) = request.header("X-CSRF-Token") {
            // Retrieve from headers
            header_token.as_str().to_string()
        } else if request.content_type() == Some(mime::FORM) {
            // Retrieve from parameters
            let form_data: JsonValue = request.body_form().await?;
            match form_data["_token"].as_str() {
                Some(t) => t.to_string(),
                None => return Ok(Response::builder(StatusCode::BadRequest).body("CSRF token not found").build()),
            }
        } else {
            return Ok(Response::builder(StatusCode::BadRequest).body("CSRF token not found").build());
        };

        // Decode and decrypt token
        let decoded_buffer;
        let (nonce, cipher_text) = match BASE64.decode(token.as_bytes()) {
            Ok(v) if v.len() >= 12 => {
                decoded_buffer = v;
                (&decoded_buffer[..12], &decoded_buffer[12..])
            }
            _ => {
                return Ok(Response::builder(StatusCode::BadRequest)
                    .body("Failed to decode CSRF token")
                    .build())
            }
        };
        let nonce_array = GenericArray::from_slice(nonce);
        let decrypted = match self.cipher.decrypt(nonce_array, cipher_text) {
            Ok(plain) => plain,
            Err(_) => {
                return Ok(Response::builder(StatusCode::BadRequest)
                    .body("Failed to decrypt CSRF token")
                    .build());
            }
        };
        let params: Vec<_> = str::from_utf8(&decrypted)?.split_whitespace().collect();

        // Verify session ID
        let sid = request.session().id();
        if params.get(0) != Some(&sid) {
            return Ok(Response::builder(StatusCode::BadRequest).body("Invalid CSRF token data").build());
        }

        // Verify request timestamp
        let token_time = params.get(1).map(|s| s.parse().ok()).flatten().unwrap_or(0);
        let now = Local::now().timestamp();
        if now - token_time > self.token_expiary as i64 {
            return Ok(Response::builder(StatusCode::BadRequest).body("Invalid CSRF token data").build());
        }

        let response = next.run(request).await;
        Ok(response)
    }
}

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
    let nonce = random::<[u8; 12]>();
    let nonce = GenericArray::from_slice(&nonce);
    let plain_text = format!("{} {}", session.id(), Local::now().timestamp().to_string());

    let cipher_bytes = cipher
        .encrypt(&nonce, plain_text.as_bytes())
        .map_err(|_| format_err!("Failed to encrypt token"))?;
    Ok(BASE64.encode(&cipher_bytes))
}

/// Pops existing flash messages and inserts new ones.
pub fn swap_flashes(session: &mut Session, new_flashes: Vec<Flash>) -> Result<Vec<Flash>> {
    let old_flashes = session.get("flash_messages").unwrap_or_default();
    session.insert("flash_messages", new_flashes)?;

    Ok(old_flashes)
}
