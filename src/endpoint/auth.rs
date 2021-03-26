//! Contains authentication endpoints.

use crate::{
    action::session::{delete_account, set_account, swap_flashes, Account, Common, Flash},
    application::State,
    ensure_login, template, validate_form,
};

use async_std::sync::Arc;

use anyhow::format_err;
use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};
use log::debug;
use serde::Deserialize;
use tide::{
    http::{mime, StatusCode},
    Redirect, Request, Response, Result as TideResult,
};
use yarte::Template;

/// `GET /signin`
/// Renders sign in page.
pub async fn render_signin(mut request: Request<Arc<State>>) -> TideResult {
    debug!("Rendering /signin");

    let state = request.state().clone();
    let session = request.session_mut();

    let info = template::PageInfo::new(&state, "/signin")?.with_title("Sign in");
    let common = Common::new(&state, session, vec![])?;
    Ok(Response::builder(StatusCode::Ok)
        .content_type(mime::HTML)
        .body(template::Signin { info, common }.call()?)
        .build())
}

/// `POST /signin`
/// Performs sign in.
pub async fn signin(mut request: Request<Arc<State>>) -> TideResult {
    debug!("Performing /signin");

    #[derive(Debug, Deserialize)]
    struct Parameters {
        _token: String,
        username: String,
        password: String,
    }

    let mut flashes = vec![];
    let state = request.state().clone();
    let params = validate_form!(Parameters, request, "/signin");
    let session = request.session_mut();

    // Verify username
    if &params.username != &state.account.0 {
        flashes.push(Flash::Error("User not found".into()));
        swap_flashes(session, flashes)?;
        return Ok(Redirect::new("/signin").into());
    }

    // Verify password
    let argon2 = Argon2::default();
    let password_hash = PasswordHash::new(&state.account.1).map_err(|_| format_err!("Invalid password hash"))?;
    match argon2.verify_password(params.password.as_bytes(), &password_hash) {
        Ok(()) => {}
        Err(_) => {
            flashes.push(Flash::Error("User not found".into()));
            swap_flashes(session, flashes)?;
            return Ok(Redirect::new("/signin").into());
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
    session.regenerate();

    Ok(Redirect::new("/").into())
}

/// `DELETE /signout`
/// Performs sign out.
pub async fn signout(mut request: Request<Arc<State>>) -> TideResult {
    debug!("Performing /signout");
    ensure_login!(request);

    let mut flashes = vec![];
    let session = request.session_mut();

    delete_account(session)?;
    flashes.push(Flash::Info(format!("Signed out successfully.")));
    swap_flashes(session, flashes)?;

    Ok(Redirect::new("/").into())
}
