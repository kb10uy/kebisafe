//! API endpoints

use crate::{
    action::{
        database::{fetch_media, reserve_media_record},
        media::{create_thumbnail, save_image, validate_image_file},
    },
    api::schema::{ErrorResponse, ShowMediaQuery, ShowMediaResponse, UploadMediaQuery},
    application::State,
};

use async_std::{sync::Arc, task::spawn};

use image::ImageFormat;
use log::debug;
use tide::{
    http::{mime, StatusCode},
    Request, Response, Result as TideResult,
};

/// `GET /api/show`
pub async fn show(request: Request<Arc<State>>) -> TideResult {
    debug!("API endpoint: GET /api/show");

    let query: ShowMediaQuery = request.query()?;
    let state = request.state().clone();

    let media_record = match fetch_media(&state.pool, &query.hash_id).await? {
        Some(m) => m,
        None => {
            return Ok(ErrorResponse::build(
                StatusCode::NotFound,
                format!("Media #{} not found", query.hash_id),
            )?);
        }
    };

    Ok(Response::builder(StatusCode::Ok)
        .content_type(mime::JSON)
        .body(serde_json::to_string(&ShowMediaResponse::from_media_record(
            &state,
            &media_record,
        )?)?)
        .build())
}

pub async fn upload(mut request: Request<Arc<State>>) -> TideResult {
    debug!("API endpoint: GET /api/upload");

    let query: UploadMediaQuery = request.query()?;
    let state = request.state().clone();
    let body = request.body_bytes().await?;

    let validated_image = match validate_image_file(&query.filename, &body) {
        Ok(image) => image,
        Err(e) => {
            return Ok(ErrorResponse::build(
                StatusCode::NotFound,
                format!("Failed to validate image: {}", e),
            )?);
        }
    };
    let thumbnail = create_thumbnail(&validated_image.image);

    let record = reserve_media_record(
        &state.pool,
        &validated_image,
        thumbnail.is_some(),
        query.private.unwrap_or_default(),
    )
    .await?;
    let filename = state.media_root.join(format!("{}.{}", record.hash_id, record.extension));
    let thumb_filename = state.media_root.join(format!("thumbnails/{}.jpg", record.hash_id));
    if let Some(thumb) = thumbnail {
        spawn(async move { save_image(&thumb, ImageFormat::Jpeg, &thumb_filename) }).await?;
        spawn(async move { save_image(&validated_image.image, validated_image.format, &filename) }).await?;
    } else {
        spawn(async move { save_image(&validated_image.image, validated_image.format, &filename) }).await?;
    }

    Ok(Response::builder(StatusCode::Ok)
        .content_type(mime::JSON)
        .body(serde_json::to_string(&ShowMediaResponse::from_media_record(&state, &record)?)?)
        .build())
}
