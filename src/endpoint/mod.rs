use crate::{application::State, template};

use std::io::{prelude::*, Cursor};

use log::info;
use multipart::server::{Multipart, MultipartField};
use tide::{
    http::{mime, StatusCode},
    Request, Response, Result as TideResult,
};
use yarte::Template;

pub async fn index(_request: Request<State>) -> TideResult {
    Ok(Response::builder(StatusCode::Ok)
        .content_type(mime::HTML)
        .body(template::Index { pictures: vec![] }.call()?)
        .build())
}

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
