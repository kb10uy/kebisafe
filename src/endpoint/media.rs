//! Contains media endpoints.

use crate::{
    action::{
        database::reserve_media_record,
        media::{create_thumbnail, validate_image_file},
        session::{swap_flashes, Common, Flash},
    },
    application::State,
    template,
};

use async_std::sync::Arc;
use image::GenericImageView;
use std::io::{prelude::*, Cursor};

use anyhow::bail;
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
        Some(b) if content_type.essence() == mime::MULTIPART_FORM.essence() => b.as_str(),
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
    let (bytes, filename) = match file {
        Some(file) => file,
        None => return Ok(Response::builder(StatusCode::BadRequest).body("Invalid multipart request").build()),
    };

    let state = request.state().clone();
    let session = request.session_mut();
    let validated_image = match validate_image_file(&filename, &bytes) {
        Ok(image) => image,
        Err(e) => {
            let flashes = vec![Flash::Error(format!("Failed to validate image: {}", e))];
            swap_flashes(session, flashes)?;
            return Ok(Redirect::new("/").into());
        }
    };
    let thumbnail = create_thumbnail(&validated_image.image);

    let record = reserve_media_record(&state.pool, &validated_image, thumbnail.is_some()).await?;

    let (width, height) = validated_image.image.dimensions();
    let message = format!(
        r#"
        Image: {}
        Type: {:?}
        Size: {}x{}
        Thumbnail: {}
    "#,
        filename,
        validated_image.format,
        width,
        height,
        thumbnail.is_some()
    );

    Ok(Response::builder(StatusCode::Ok).content_type(mime::PLAIN).body(message).build())
}
