use anyhow::Result;
use image::{DynamicImage, ImageReader};
use std::io::Cursor;

/// Decode arbitrary image bytes into a `DynamicImage`.
///
/// Format is auto-detected from the byte signature; the file extension is not
/// consulted, which matches how archives may rename / re-extension entries.
pub fn decode(bytes: &[u8]) -> Result<DynamicImage> {
    let img = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()?
        .decode()?;
    Ok(img)
}

/// Resize to fit within `max_w` × `max_h` using Lanczos3.
pub fn resize_lanczos3(img: &DynamicImage, max_w: u32, max_h: u32) -> DynamicImage {
    img.resize(max_w, max_h, image::imageops::FilterType::Lanczos3)
}
