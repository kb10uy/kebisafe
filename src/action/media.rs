//! Contains media manipulations.

use async_std::path::PathBuf;

use anyhow::{bail, Result};
use image::{DynamicImage, ImageFormat};
use mime_guess::MimeGuess;

const ALLOWED_TYPES: &[&str] = &["image/png", "image/jpeg", "image/gif", "image/webp"];

#[derive(Debug)]
pub struct RegisteredImage {
    pub image: DynamicImage,
    pub filename: PathBuf,
}

pub fn register_file(filename: &str, data: &[u8]) -> Result<String> {
    let mime_type = match MimeGuess::from_path(filename).first() {
        Some(mime) if ALLOWED_TYPES.contains(&mime.as_ref()) => mime,
        Some(_) => bail!("Unsupported file type"),
        _ => bail!("Cannot determine file type"),
    };
    match image::guess_format(data) {
        Ok(ImageFormat::Png) if mime_type == "image/png" => (),
        Ok(ImageFormat::Jpeg) if mime_type == "image/jpeg" => (),
        Ok(ImageFormat::Gif) if mime_type == "image/gif" => (),
        Ok(ImageFormat::WebP) if mime_type == "image/webp" => (),
        _ => bail!("Unsupported image type"),
    };
    todo!();
}
