//! Contains media endpoints.

use crate::{
    action::session::{swap_flashes, Common, Flash},
    application::State,
    template,
};

use async_std::sync::Arc;
use std::io::{prelude::*, Cursor};

use multipart::server::Multipart;
use tide::{
    http::{mime, StatusCode},
    Redirect, Request, Response, Result as TideResult,
};
use yarte::Template;

/// POST `/upload`
/// Uploads a file.
pub async fn upload(mut request: Request<Arc<State>>) -> TideResult {
    let content_type = request.content_type().unwrap();
    let boundary = match content_type.param("boundary") {
        Some(b) if content_type.essence() != mime::MULTIPART_FORM.essence() => b.as_str(),
        _ => return Ok(Response::builder(StatusCode::BadRequest).body("Invalid multipart request").build()),
    };

    let body = request.body_bytes().await?;
    let mut multipart = Multipart::with_body(Cursor::new(&body[..]), boundary);

    let mut file = None;
    while let Some(mut mpf) = multipart.read_entry()? {
        let field_name = mpf.headers.name.as_ref();
        let filename = mpf.headers.filename;
        match (field_name, filename) {
            ("upload_file", Some(filename)) => {
                let mut bytes = vec![];
                mpf.data.read_to_end(&mut bytes)?;
                file = Some((bytes, filename));
            }
            _ => (),
        }
    }
    todo!();
}
