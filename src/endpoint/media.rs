//! Contains media endpoints.

use crate::{
    action::{
        database::reserve_media_record,
        media::{create_thumbnail, save_image, validate_image_file},
        session::{swap_flashes, Common, Flash},
    },
    application::State,
    template,
};

use async_std::{sync::Arc, task::spawn};
use std::io::{prelude::*, Cursor};

use anyhow::bail;
use futures::join;
use image::ImageFormat;
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
    let filename = state.media_root.join(format!("{}.{}", record.hash_id, record.extension));
    let thumb_filename = state.media_root.join(format!("thumbnails/{}.jpg", record.hash_id));
    if let Some(thumb) = thumbnail {
        spawn(async move { save_image(&thumb, ImageFormat::Jpeg, &thumb_filename) }).await?;
        spawn(async move { save_image(&validated_image.image, validated_image.format, &filename) }).await?;
    } else {
        spawn(async move { save_image(&validated_image.image, validated_image.format, &filename) }).await?;
    }

    Ok(Redirect::new(format!("/m/{}", record.hash_id)).into())
}
