//! Contains root-level endpoints.

pub(crate) mod auth;
pub(crate) mod media;

use crate::{
    action::session::{swap_flashes, Common, Flash},
    application::State,
    template,
};

use async_std::sync::Arc;
use tide::{
    http::{mime, StatusCode},
    Redirect, Request, Response, Result as TideResult,
};
use yarte::Template;

/// `GET /`
/// Index
pub async fn index(mut request: Request<Arc<State>>) -> TideResult {
    let state = request.state().clone();
    let session = request.session_mut();

    let common = Common::with_csrf_token(session, vec![], &state.cipher)?;
    Ok(Response::builder(StatusCode::Ok)
        .content_type(mime::HTML)
        .body(template::Index { common, pictures: vec![] }.call()?)
        .build())
}

pub async fn add_flash(mut request: Request<Arc<State>>) -> TideResult {
    let session = request.session_mut();

    let new_flashes = vec![Flash::Info("Sample generated".to_string())];
    swap_flashes(session, new_flashes)?;

    Ok(Redirect::new("/").into())
}
