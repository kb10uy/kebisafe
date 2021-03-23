//! Contains root-level endpoints.

pub(crate) mod auth;

use crate::{
    application::State,
    session::{swap_flashes, Common, Flash},
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

/*
async fn index_test(mut request: Request<()>) -> TideResult {
    let content_type = request.content_type().unwrap();
    if content_type.essence() != mime::MULTIPART_FORM.essence() {
        return Ok(Response::builder(StatusCode::BadRequest)
            .body("You must upload at least one file")
            .build());
    }
    let boundary = match content_type.param("boundary") {
        Some(b) => b.as_str(),
        None => return Ok(Response::builder(StatusCode::BadRequest).body("Invalid multipart request").build()),
    };

    let body = request.body_bytes().await?;
    let wrapped_body = Cursor::new(&body[..]);
    let mut multipart = Multipart::with_body(wrapped_body, boundary);

    loop {
        let part = match multipart.read_entry()? {
            Some(part) => part,
            None => break,
        };
        let MultipartField { headers, mut data } = part;

        match headers.filename {
            Some(f) => {
                let mut bytes = vec![];
                data.read_to_end(&mut bytes)?;
                info!("File {} given, size was {} bytes", f, bytes.len())
            }
            None => info!("Non-file given"),
        }
    }

    todo!();
}
*/
