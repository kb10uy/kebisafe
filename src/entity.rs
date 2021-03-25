use chrono::prelude::*;
use sqlx::prelude::*;

/// Represents a media record.
#[derive(Debug, Clone, FromRow)]
pub struct Media {
    pub hash_id: String,
    pub extension: String,
    pub has_thumbnail: bool,
    pub width: i32,
    pub height: i32,
    pub comment: Option<String>,
    pub uploaded: DateTime<Local>,
}
