//! Contains database manipulation.

use crate::{action::media::ValidatedImage, entity::Media};

use anyhow::{anyhow, Result};
use chrono::prelude::*;
use image::GenericImageView;
use log::info;
use once_cell::sync::Lazy;
use rand::prelude::*;
use sqlx::{error::DatabaseError, Error as SqlxError, PgPool};

static HASH_CHARS: Lazy<Box<[char]>> = Lazy::new(|| "0123456789abcdefghijklmnopqrstuvwxyz".chars().collect());
const HASH_MIN_LENGTH: usize = 6;
const MAX_RETRY: usize = 5;

/// Counts all records.
pub async fn fetch_records_count(pool: &PgPool) -> Result<usize> {
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM media;").fetch_one(pool).await?;
    Ok(count as usize)
}

/// Fetches a media record.
pub async fn fetch_media(pool: &PgPool, hash_id: &str) -> Result<Option<Media>> {
    let media = sqlx::query_as("SELECT * FROM media WHERE hash_id = $1;")
        .bind(hash_id)
        .fetch_optional(pool)
        .await?;

    Ok(media)
}

/// Fetches media list.
pub async fn fetch_media_list(pool: &PgPool, latest: Option<DateTime<Local>>, limit: usize) -> Result<Vec<Media>> {
    let query_str = if latest.is_some() {
        "SELECT * FROM media WHERE uploaded < $1 ORDER BY uploaded DESC LIMIT $2;"
    } else {
        "SELECT * FROM media ORDER BY uploaded DESC LIMIT $2;"
    };
    let media = sqlx::query_as(query_str).bind(latest).bind(limit as i64).fetch_all(pool).await?;

    Ok(media)
}

/// Reserves a database record for media.
pub async fn reserve_media_record(pool: &PgPool, validated_image: &ValidatedImage, thumbnail: bool) -> Result<Media> {
    let extension = validated_image
        .format
        .extensions_str()
        .get(0)
        .expect("Validated image should have extension");
    let (width, height) = validated_image.image.dimensions();

    for i in 0..MAX_RETRY {
        let length = HASH_MIN_LENGTH + i;
        info!("Attempting {}... ({} chars)", i + 1, length);

        let chars = HASH_CHARS.as_ref();
        let hash: String = chars.choose_multiple(&mut thread_rng(), length).collect();

        let query_result = sqlx::query_as(
            r#"
            INSERT INTO media (
                hash_id,
                extension,
                has_thumbnail,
                width,
                height,
                uploaded
            ) VALUES (
                $1, $2, $3, $4, $5, $6
            ) RETURNING *;
        "#,
        )
        .bind(hash)
        .bind(extension)
        .bind(thumbnail)
        .bind(width as i32)
        .bind(height as i32)
        .bind(Local::now())
        .fetch_one(pool)
        .await;

        match query_result {
            Ok(media) => return Ok(media),
            Err(SqlxError::Database(sql_err)) if is_conflicting(sql_err.as_ref()) => continue,
            Err(err) => return Err(err.into()),
        }
    }

    Err(anyhow!("Failed to create record"))
}

/// Judges whether given `DatabaseError` implies constraint errors.
fn is_conflicting(sql_err: &dyn DatabaseError) -> bool {
    // On Postgres (and MySQL), SQLSTATE 23___ represents constraint error
    // https://www.postgresql.org/docs/13/errcodes-appendix.html
    // https://dev.mysql.com/doc/mysql-errors/8.0/en/server-error-reference.html
    match sql_err.code() {
        Some(sqlstate) if sqlstate.starts_with("23") => true,
        _ => false,
    }
}
