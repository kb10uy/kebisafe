use sqlx::prelude::*;
use time::OffsetDateTime;

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
    pub uploaded: OffsetDateTime,
}

#[allow(dead_code)]
impl Media {
    /// Returns approximate representation of filesize.
    pub fn filesize_str(&self) -> String {
        if self.filesize <= 1 {
            format!("{} byte", self.filesize)
        } else if self.filesize < 1024 {
            format!("{} bytes", self.filesize)
        } else if self.filesize < 1048576 {
            format!("{:.2} KiB", self.filesize as f64 / 1024.0)
        } else {
            format!("{:.2} MiB", self.filesize as f64 / 1048576.0)
        }
    }
}
