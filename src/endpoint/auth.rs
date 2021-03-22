use crate::{
    application::State,
    session::{swap_flashes, Flash},
};

use async_std::sync::Arc;

use anyhow::format_err;
use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};
use serde::Deserialize;
use tide::{Redirect, Request, Result as TideResult};

pub async fn signin(mut request: Request<Arc<State>>) -> TideResult {
    #[derive(Debug, Deserialize)]
    struct Parameters {
        username: String,
        password: String,
    }

    let mut flashes = vec![];
    let state = request.state().clone();
    let params: Parameters = request.body_form().await?;
    let session = request.session_mut();

    let argon2 = Argon2::default();
    let password_hash = PasswordHash::new(&state.account.1).map_err(|_| format_err!("Invalid password hash"))?;
    if &params.username != &state.account.0 {
        flashes.push(Flash::Error("User not found".into()));
    }
    match argon2.verify_password(params.password.as_bytes(), &password_hash) {
        Ok(()) => {
            flashes.push(Flash::Info(format!("Welcome back, {}", params.username)));
        }
        Err(_) => {
            flashes.push(Flash::Error("User not found".into()));
        }
    }

    swap_flashes(session, flashes)?;
    return Ok(Redirect::new("/").into());
}
