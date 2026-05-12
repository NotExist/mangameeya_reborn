use anyhow::{Context, Result};
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

/// Resize to fit within `max_w` × `max_h` using Lanczos3, preserving aspect.
///
/// Uses `fast_image_resize` (SIMD on x86/aarch64) — typically 5–10× faster
/// than `image::imageops::resize`.
pub fn resize_lanczos3(img: &DynamicImage, max_w: u32, max_h: u32) -> Result<DynamicImage> {
    use fast_image_resize::images::{Image as FirImage, ImageRef as FirImageRef};
    use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};

    let (sw, sh) = (img.width(), img.height());
    let (tw, th) = fit_within(sw, sh, max_w, max_h);
    if (tw, th) == (sw, sh) {
        return Ok(img.clone());
    }

    let src_rgb = img.to_rgb8();
    let src = FirImageRef::new(sw, sh, src_rgb.as_raw(), PixelType::U8x3)
        .context("source buffer shape")?;
    let mut dst = FirImage::new(tw, th, PixelType::U8x3);
    let mut resizer = Resizer::new();
    let opts = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3));
    resizer
        .resize(&src, &mut dst, &opts)
        .context("fir resize")?;

    let buf =
        image::RgbImage::from_raw(tw, th, dst.into_vec()).context("destination buffer shape")?;
    Ok(DynamicImage::ImageRgb8(buf))
}

/// Largest `(w, h)` that fits inside `max_w × max_h` and preserves aspect.
fn fit_within(sw: u32, sh: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    if sw <= max_w && sh <= max_h {
        return (sw, sh);
    }
    let r = (max_w as f64 / sw as f64).min(max_h as f64 / sh as f64);
    let tw = ((sw as f64) * r).round() as u32;
    let th = ((sh as f64) * r).round() as u32;
    (tw.max(1), th.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_keeps_small_unchanged() {
        assert_eq!(fit_within(100, 200, 1000, 1000), (100, 200));
    }

    #[test]
    fn fit_scales_down_preserving_aspect() {
        let (w, h) = fit_within(2400, 3400, 1920, 1080);
        // 3400 ratio is tighter: 1080/3400 ≈ 0.3176
        // Expected width ≈ 2400 * 0.3176 ≈ 762
        assert_eq!(h, 1080);
        assert!((760..=764).contains(&w), "got {}", w);
    }

    #[test]
    fn fit_scales_down_landscape() {
        let (w, h) = fit_within(6000, 4000, 1920, 1080);
        // 1920/6000 = 0.32 tighter than 1080/4000 = 0.27 — wait, 0.27 is smaller
        // so 1080/4000 wins: width = 6000 * 0.27 = 1620
        assert_eq!(h, 1080);
        assert!((1618..=1622).contains(&w), "got {}", w);
    }
}
