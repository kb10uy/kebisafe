//! Contains authentication endpoints.

use crate::{
    application::State,
    csrf_protect,
    session::{set_account, swap_flashes, Account, Flash},
};

use async_std::sync::Arc;

use anyhow::format_err;
use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};
use serde::Deserialize;
use tide::{Redirect, Request, Result as TideResult};

/// `POST /signin`
/// Performs sign in.
pub async fn signin(mut request: Request<Arc<State>>) -> TideResult {
    #[derive(Debug, Deserialize)]
    struct Parameters {
        _token: String,
        username: String,
        password: String,
    }

    let state = request.state().clone();
    let params: Parameters = request.body_form().await?;
    csrf_protect!(request, &params._token);

    let mut flashes = vec![];
    let session = request.session_mut();

    // Verify username
    if &params.username != &state.account.0 {
        flashes.push(Flash::Error("User not found".into()));
        return Ok(Redirect::new("/").into());
    }

    // Verify password
    let argon2 = Argon2::default();
    let password_hash = PasswordHash::new(&state.account.1).map_err(|_| format_err!("Invalid password hash"))?;
    match argon2.verify_password(params.password.as_bytes(), &password_hash) {
        Ok(()) => {}
        Err(_) => {
            flashes.push(Flash::Error("User not found".into()));
            return Ok(Redirect::new("/").into());
        }
    }

    set_account(
        session,
        Account {
            name: state.account.0.clone(),
        },
    )?;
    flashes.push(Flash::Info(format!("Welcome back, {}", params.username)));
    swap_flashes(session, flashes)?;

    Ok(Redirect::new("/").into())
}
