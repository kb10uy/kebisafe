//! Contains media manipulations.

use async_std::path::Path;

use anyhow::{bail, Result};
use image::{imageops::FilterType, DynamicImage, GenericImageView, ImageFormat};
use mime_guess::MimeGuess;

const ALLOWED_TYPES: &[(&str, ImageFormat)] = &[
    ("image/png", ImageFormat::Png),
    ("image/jpeg", ImageFormat::Jpeg),
    ("image/gif", ImageFormat::Gif),
    ("image/webp", ImageFormat::WebP),
];

const THUMBNAIL_WIDTH: u32 = 320;
const THUMBNAIL_HEIGHT: u32 = 180;

#[derive(Debug)]
pub struct ValidatedImage {
    pub image: DynamicImage,
    pub format: ImageFormat,
}

/// Validates input filename and blob.
/// Returns decoded image and extension if succeeded.
pub fn validate_image_file(filename: impl AsRef<Path>, data: &[u8]) -> Result<ValidatedImage> {
    let path = filename.as_ref();
    let extension = match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => ext,
        None => bail!("Cannot determine file type"),
    };

    let detected_type = match MimeGuess::from_ext(extension).first() {
        Some(mime) => {
            if let Some(ty) = ALLOWED_TYPES.iter().find(|ty| ty.0 == mime.as_ref()) {
                ty.1
            } else {
                bail!("Unsupported file type");
            }
        }
        _ => bail!("Cannot determine file type"),
    };
    let format = match image::guess_format(data) {
        Ok(f) if f == detected_type => f,
        _ => bail!("Unsupported image type"),
    };
    let image = image::load_from_memory_with_format(data, format)?;

    Ok(ValidatedImage { image, format })
}

/// Creates thumbnail image.
/// If original image is small enough, return `None`.
pub fn create_thumbnail(original_image: &DynamicImage) -> Option<DynamicImage> {
    let (width, height) = original_image.dimensions();

    if width <= THUMBNAIL_WIDTH && height <= THUMBNAIL_HEIGHT {
        // Original size will fit in thumbnail size
        return None;
    } else if width <= THUMBNAIL_WIDTH {
        // Clip top and bottom
        let top_half = (height - THUMBNAIL_HEIGHT) / 2;
        let cropped = original_image.crop_imm(0, top_half, width, THUMBNAIL_HEIGHT);
        Some(cropped)
    } else if height <= THUMBNAIL_HEIGHT {
        // Clip left and right
        let left_half = (width - THUMBNAIL_WIDTH) / 2;
        let cropped = original_image.crop_imm(left_half, 0, THUMBNAIL_WIDTH, height);
        Some(cropped)
    } else {
        // Scale down
        let scaled = original_image.resize(THUMBNAIL_WIDTH, THUMBNAIL_HEIGHT, FilterType::Triangle);
        Some(scaled)
    }
}
