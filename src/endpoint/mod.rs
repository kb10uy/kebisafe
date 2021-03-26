//! Contains root-level endpoints.

pub(crate) mod auth;
pub(crate) mod media;

use crate::{
    action::{database::fetch_records_count, session::Common},
    application::State,
    template,
};

use async_std::sync::Arc;
use log::debug;
use tide::{
    http::{mime, StatusCode},
    Request, Response, Result as TideResult,
};
use yarte::Template;

/// `GET /`
/// Index
pub async fn index(mut request: Request<Arc<State>>) -> TideResult {
    debug!("Rendering /");

    let state = request.state().clone();
    let session = request.session_mut();

    let pictures_count = fetch_records_count(&state.pool).await?;
    let info = template::PageInfo::new(&state, "/")?;
    let common = Common::new(&state, session, vec![])?;
    Ok(Response::builder(StatusCode::Ok)
        .content_type(mime::HTML)
        .body(
            template::Index {
                info,
                common,
                pictures_count,
            }
            .call()?,
        )
        .build())
}
