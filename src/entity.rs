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

    /// Whether media is private
    pub is_private: bool,

    /// Media width
    pub width: i32,

    /// Media height
    pub height: i32,

    /// Filesize of media
    pub filesize: i32,

    /// Comment for media
    pub comment: Option<String>,

    /// Uploaded date
    pub uploaded: DateTime<Local>,
}
