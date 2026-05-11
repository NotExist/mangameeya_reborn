use image::{ExtendedColorType, ImageBuffer, ImageEncoder, Rgb};

/// Synthesise a unique-ish RGB image and JPEG-encode it.
///
/// Uses a per-page seed to drive a fast XorShift PRNG plus a deterministic
/// gradient, so each page is visually distinct from its neighbours and from
/// runs with a different seed range. JPEG quality 85 mirrors real-world manga
/// scans closely enough that decoder timings stay representative.
pub fn synthesize_page(seed: u32, w: u32, h: u32) -> Vec<u8> {
    let seed = seed.max(1);
    let mut buf = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(w, h);
    let r_off = (seed & 0xff) as u8;
    let g_off = ((seed >> 8) & 0xff) as u8;
    let b_off = ((seed >> 16) & 0xff) as u8;
    let mut rng = seed.wrapping_mul(2_654_435_761);

    for y in 0..h {
        for x in 0..w {
            let nx = x as f32 / w as f32;
            let ny = y as f32 / h as f32;
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            let n = (rng & 0x1f) as u8;
            let r = ((nx * 255.0) as u8).wrapping_add(r_off).wrapping_add(n);
            let g = ((ny * 255.0) as u8).wrapping_add(g_off).wrapping_add(n);
            let b = (((nx + ny) * 127.0) as u8)
                .wrapping_add(b_off)
                .wrapping_add(n);
            buf.put_pixel(x, y, Rgb([r, g, b]));
        }
    }

    let mut out: Vec<u8> = Vec::with_capacity((w * h / 4) as usize);
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85);
    encoder
        .write_image(buf.as_raw(), w, h, ExtendedColorType::Rgb8)
        .expect("synthesised RGB buffer is always JPEG-encodable");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synth_is_deterministic() {
        let a = synthesize_page(42, 100, 100);
        let b = synthesize_page(42, 100, 100);
        assert_eq!(a, b);
    }

    #[test]
    fn synth_differs_by_seed() {
        let a = synthesize_page(1, 100, 100);
        let b = synthesize_page(2, 100, 100);
        assert_ne!(a, b);
    }

    #[test]
    fn synth_is_valid_jpeg() {
        let bytes = synthesize_page(1, 64, 64);
        crate::decode::decode(&bytes).expect("decodes");
    }
}
