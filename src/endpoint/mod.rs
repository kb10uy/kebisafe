//! Contains root-level endpoints.

pub(crate) mod auth;
pub(crate) mod media;

use crate::{
    action::{database::fetch_records_count, session::Common},
    application::State,
    template,
};

use async_std::sync::Arc;
use tide::{
    http::{mime, StatusCode},
    Request, Response, Result as TideResult,
};
use yarte::Template;

/// `GET /`
/// Index
pub async fn index(mut request: Request<Arc<State>>) -> TideResult {
    let state = request.state().clone();
    let session = request.session_mut();

    let pictures_count = fetch_records_count(&state.pool).await?;
    let common = Common::with_csrf_token(session, vec![], &state.cipher)?;
    Ok(Response::builder(StatusCode::Ok)
        .content_type(mime::HTML)
        .body(template::Index { common, pictures_count }.call()?)
        .build())
}
