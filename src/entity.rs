use chrono::prelude::*;
use sqlx::prelude::*;

/// Represents a media record.
#[derive(Debug, Clone, FromRow)]
pub struct Media {
    /// Short hash ID
    pub hash_id: String,

    /// Media extension
    pub extension: String,

    /// Whether media has dedicated thumbnail
    pub has_thumbnail: bool,

    /// media width
    pub width: i32,

    /// media height
    pub height: i32,

    /// Comment for media
    pub comment: Option<String>,

    /// Uploaded date
    pub uploaded: DateTime<Local>,
}
