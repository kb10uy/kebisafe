//! API schema types

use crate::{application::State, entity::Media};

use anyhow::Result;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use tide::{http::StatusCode, Response};
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ErrorResponse {
    pub message: String,
}

impl ErrorResponse {
    pub fn build(status: StatusCode, message: impl Into<String>) -> Result<Response> {
        let response = Response::builder(status)
            .body(serde_json::to_string(&ErrorResponse { message: message.into() })?)
            .build();

        Ok(response)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ShowMediaQuery {
    pub hash_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShowMediaResponse {
    pub url: Url,
    pub permalink: Url,
    pub hash_id: String,
    pub width: usize,
    pub height: usize,
    pub filesize: usize,
    pub private: bool,
    pub comment: Option<String>,
    pub uploaded: DateTime<Local>,
}

impl ShowMediaResponse {
    /// Constructs from `Meida`
    pub fn from_media_record(state: &State, media: &Media) -> Result<ShowMediaResponse> {
        Ok(ShowMediaResponse {
            url: state.hosted_at.join(&format!("/m/{}", media.hash_id))?,
            permalink: state.hosted_at.join(&format!("/media/{}.{}", media.hash_id, media.extension))?,
            hash_id: media.hash_id.clone(),
            width: media.width as usize,
            height: media.height as usize,
            filesize: media.filesize as usize,
            private: media.is_private,
            comment: media.comment.clone(),
            uploaded: media.uploaded,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct UploadMediaQuery {
    pub filename: String,
    pub private: Option<bool>,
    pub comment: Option<String>,
}
