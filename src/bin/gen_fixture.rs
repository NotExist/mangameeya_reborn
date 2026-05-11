//! Generate a synthetic manga-shaped zip fixture for benchmarking.
//!
//! Usage:
//!   gen_fixture <output.zip> [normal_count=220] [extreme_count=30]

use anyhow::{Context, Result};
use mangameeya_reborn::fixture::synthesize_page;
use rayon::prelude::*;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

const NORMAL_W: u32 = 2400;
const NORMAL_H: u32 = 3400;
const EXTREME_W: u32 = 6000;
const EXTREME_H: u32 = 4000;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let out = PathBuf::from(
        args.get(1)
            .cloned()
            .unwrap_or_else(|| "bench-fixture.zip".into()),
    );
    let n_normal: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(220);
    let n_extreme: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(30);

    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
    }

    let total = n_normal + n_extreme;
    eprintln!(
        "[gen_fixture] target={} normal={} extreme={} total={}",
        out.display(),
        n_normal,
        n_extreme,
        total
    );

    let start = Instant::now();

    let pages: Vec<(String, Vec<u8>)> = (0..total)
        .into_par_iter()
        .map(|i| {
            let (w, h) = if i < n_normal {
                (NORMAL_W, NORMAL_H)
            } else {
                (EXTREME_W, EXTREME_H)
            };
            let name = format!("page_{:04}.jpg", i + 1);
            let bytes = synthesize_page(i + 1, w, h);
            (name, bytes)
        })
        .collect();
    eprintln!(
        "[gen_fixture] synthesised {} pages in {:.1}ms",
        total,
        start.elapsed().as_secs_f64() * 1000.0
    );

    let file =
        File::create(&out).with_context(|| format!("creating {}", out.display()))?;
    let mut zip = ZipWriter::new(file);
    // JPEG is already compressed; storing without deflate is faster and barely
    // larger.
    let opts =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    let mut total_bytes: usize = 0;
    for (name, bytes) in &pages {
        zip.start_file(name, opts)?;
        zip.write_all(bytes)?;
        total_bytes += bytes.len();
    }
    zip.finish()?;

    let total_mb = total_bytes as f64 / (1024.0 * 1024.0);
    eprintln!(
        "[gen_fixture] done: {} pages, {:.1} MB raw, {} written in {:.1}ms",
        total,
        total_mb,
        out.display(),
        start.elapsed().as_secs_f64() * 1000.0
    );

    Ok(())
}
