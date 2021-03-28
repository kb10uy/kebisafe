//! Contains media endpoints.

use crate::{
    action::{
        database::{fetch_media, fetch_media_list, remove_media_record, reserve_media_record, update_media_record},
        media::{create_thumbnail, save_image, validate_image_file},
        session::{swap_flashes, Common, Flash},
    },
    application::State,
    ensure_login, validate_form,
    web::{multipart::MultipartData, template, RequestPreParseExt},
};

use async_std::{fs, sync::Arc, task::spawn};

use image::ImageFormat;
use log::debug;
use serde::Deserialize;
use tide::{
    http::{mime, StatusCode},
    Redirect, Request, Response, Result as TideResult,
};
use url::Url;
use yarte::Template;

const MEDIA_LIST_COUNT: usize = 50;

/// `GET /m/`
/// Shows a media.
pub async fn list_media(mut request: Request<Arc<State>>) -> TideResult {
    debug!("Rendering /m/");

    let state = request.state().clone();
    let session = request.session_mut();

    let info = template::PageInfo::new(&state, "/m/")?.with_title("Recently uploaded media");
    let common = Common::new(&state, session, vec![])?;
    let media_list = fetch_media_list(&state.pool, None, MEDIA_LIST_COUNT).await?;
    Ok(Response::builder(StatusCode::Ok)
        .content_type(mime::HTML)
        .body(template::MediaIndex { info, common, media_list }.call()?)
        .build())
}

/// `GET /m/:hash_id`
/// Shows a media.
pub async fn media(mut request: Request<Arc<State>>) -> TideResult {
    debug!("Rendering /m/:id");

    let state = request.state().clone();
    let hash_id = request.param("hash_id").expect("hash_id must be set").to_string();
    let session = request.session_mut();

    let media_record = match fetch_media(&state.pool, &hash_id).await? {
        Some(m) => m,
        None => {
            let flashes = vec![Flash::Error(format!("Media {} not found", hash_id))];
            swap_flashes(session, flashes)?;
            return Ok(Redirect::new("/").into());
        }
    };

    let common = Common::new(&state, session, vec![])?;
    let info = template::PageInfo::new(&state, &format!("/m/{}", media_record.hash_id))?
        .with_title(&format!("Media #{}", media_record.hash_id))
        .with_description(media_record.comment.as_deref().unwrap_or("<No comment>"))
        .with_thumbnail(&Url::parse(&common.permalink_thumbnail(&media_record))?);
    Ok(Response::builder(StatusCode::Ok)
        .content_type(mime::HTML)
        .body(
            template::MediaShow {
                info,
                common,
                media: media_record,
            }
            .call()?,
        )
        .build())
}

/// POST `/upload`
/// Uploads a file.
pub async fn upload(mut request: Request<Arc<State>>) -> TideResult {
    debug!("Performing /upload");
    ensure_login!(request);

    let multipart = request.body_parsed_multipart();
    let private: bool = multipart
        .get("private")
        .and_then(|v| v.as_str())
        .and_then(|v| v.parse().ok())
        .unwrap_or_default();
    let (filename, bytes) = match multipart.get("upload_file") {
        Some(MultipartData::File(filename, bytes)) => (filename, bytes),
        _ => return Ok(Response::builder(StatusCode::BadRequest).body("Invalid multipart request").build()),
    };

    let state = request.state().clone();

    let validated_image = match validate_image_file(filename, bytes) {
        Ok(image) => image,
        Err(e) => {
            let session = request.session_mut();
            let flashes = vec![Flash::Error(format!("Failed to validate image: {}", e))];
            swap_flashes(session, flashes)?;
            return Ok(Redirect::new("/").into());
        }
    };
    let thumbnail = create_thumbnail(&validated_image.image);

    let record = reserve_media_record(&state.pool, &validated_image, thumbnail.is_some(), private).await?;
    let filename = state.media_root.join(format!("{}.{}", record.hash_id, record.extension));
    let thumb_filename = state.media_root.join(format!("thumbnails/{}.jpg", record.hash_id));
    if let Some(thumb) = thumbnail {
        spawn(async move { save_image(&thumb, ImageFormat::Jpeg, &thumb_filename) }).await?;
        spawn(async move { save_image(&validated_image.image, validated_image.format, &filename) }).await?;
    } else {
        spawn(async move { save_image(&validated_image.image, validated_image.format, &filename) }).await?;
    }

    let session = request.session_mut();
    let flashes = vec![Flash::Info(format!(
        "Media has been uploaded successfully! ID is {}",
        record.hash_id
    ))];
    swap_flashes(session, flashes)?;
    Ok(Redirect::new(format!("/m/{}", record.hash_id)).into())
}

/// PATCH `/m/:hash_id`
/// Updates media information.
pub async fn update(mut request: Request<Arc<State>>) -> TideResult {
    #[derive(Deserialize)]
    struct Parameters {
        comment: String,
        private: Option<bool>,
    }

    debug!("Performing PATCH /m/:hash_id");
    ensure_login!(request);

    let state = request.state().clone();
    let hash_id = request.param("hash_id").expect("hash_id must be set").to_string();
    let params = validate_form!(Parameters, request, "/");
    let session = request.session_mut();

    let media_record = match fetch_media(&state.pool, &hash_id).await? {
        Some(m) => m,
        None => {
            let flashes = vec![Flash::Error(format!("Media {} not found", hash_id))];
            swap_flashes(session, flashes)?;
            return Ok(Redirect::new("/").into());
        }
    };
    let new_record = update_media_record(
        &state.pool,
        &media_record.hash_id,
        params.private.unwrap_or_default(),
        &params.comment,
    )
    .await?;

    let flashes = vec![Flash::Info(format!("Media information has been updated successfully."))];
    swap_flashes(session, flashes)?;
    Ok(Redirect::new(format!("/m/{}", new_record.hash_id)).into())
}

/// DELETE `/m/:hash_id`
/// Deletes a file.
pub async fn delete(mut request: Request<Arc<State>>) -> TideResult {
    debug!("Performing DELETE /m/:hash_id");
    ensure_login!(request);

    let state = request.state().clone();
    let hash_id = request.param("hash_id").expect("hash_id must be set").to_string();
    let session = request.session_mut();

    let media_record = match fetch_media(&state.pool, &hash_id).await? {
        Some(m) => m,
        None => {
            let flashes = vec![Flash::Error(format!("Media {} not found", hash_id))];
            swap_flashes(session, flashes)?;
            return Ok(Redirect::new("/").into());
        }
    };

    let filename = state
        .media_root
        .join(format!("{}.{}", media_record.hash_id, media_record.extension));
    fs::remove_file(&filename).await?;
    if media_record.has_thumbnail {
        let thumb_filename = state.media_root.join(format!("thumbnails/{}.jpg", media_record.hash_id));
        fs::remove_file(&thumb_filename).await?;
    }
    remove_media_record(&state.pool, &media_record.hash_id).await?;

    let flashes = vec![Flash::Info(format!("Media has been deleted successfully."))];
    swap_flashes(session, flashes)?;
    Ok(Redirect::new("/").into())
}
