//! Phase 1 CPU spike — open a zip fixture, drive it through decode + resize,
//! emit a structured JSON report. Pure CPU; no GPU, no display, no input.
//!
//! Usage:
//!   spike [fixture.zip]
//!
//! Output: pretty-printed JSON to stdout. Log lines go to stderr.

use anyhow::Result;
use mangameeya_reborn::{archive::ZipPageSource, decode, metrics::*};
use rayon::prelude::*;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use sysinfo::{ProcessesToUpdate, System, get_current_pid};

fn main() -> Result<()> {
    let process_start = Instant::now();

    let args: Vec<String> = std::env::args().collect();
    let fixture_path = PathBuf::from(
        args.get(1)
            .cloned()
            .unwrap_or_else(|| "bench-fixture.zip".into()),
    );

    let fixture_size = std::fs::metadata(&fixture_path)?.len();
    eprintln!(
        "[spike] start: fixture {} ({:.1} MB)",
        fixture_path.display(),
        fixture_size as f64 / (1024.0 * 1024.0)
    );

    // ---- Phase A: open zip ----
    let t = Instant::now();
    let mut src = ZipPageSource::open(&fixture_path)?;
    let zip_open_ms = t.elapsed().as_secs_f64() * 1000.0;
    let page_count = src.page_count();
    eprintln!("[spike] opened {} entries in {:.2}ms", page_count, zip_open_ms);
    assert!(page_count > 0, "fixture has no pages");

    // ---- Phase B: first-page cold decode (proxy for startup → first frame) ----
    let t = Instant::now();
    let bytes = src.page_bytes(0)?;
    let first_img = decode::decode(&bytes)?;
    let first_page_decode_ms = t.elapsed().as_secs_f64() * 1000.0;
    let startup_to_first_page_ms = process_start.elapsed().as_secs_f64() * 1000.0;
    eprintln!(
        "[spike] first page: decode {:.2}ms, startup→ready {:.2}ms",
        first_page_decode_ms, startup_to_first_page_ms
    );
    drop(first_img);

    // ---- Phase C: parallel preload of first 5 pages (simulates initial cache fill) ----
    let preload_bytes: Vec<Vec<u8>> = (0..5.min(page_count))
        .map(|i| src.page_bytes(i))
        .collect::<Result<_>>()?;
    let t = Instant::now();
    let decoded: Vec<image::DynamicImage> = preload_bytes
        .par_iter()
        .map(|b| decode::decode(b))
        .collect::<Result<_>>()?;
    let parallel_preload_5_pages_ms = t.elapsed().as_secs_f64() * 1000.0;
    eprintln!(
        "[spike] parallel preload 5 pages: {:.2}ms",
        parallel_preload_5_pages_ms
    );

    // Memory snapshot with 5 decoded pages live
    let pid = get_current_pid().expect("pid");
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
    let working_set_after_5_pages_mb = sys
        .process(pid)
        .map(|p| p.memory() as f64 / (1024.0 * 1024.0))
        .unwrap_or(0.0);
    eprintln!(
        "[spike] working set after 5 pages: {:.1} MB",
        working_set_after_5_pages_mb
    );
    drop(decoded);

    // ---- Phase D: stream-decode all pages, gather distribution ----
    let mut decode_times = Vec::with_capacity(page_count);
    let mut resize_times = Vec::with_capacity(page_count);
    let mut peak_rss_mb = working_set_after_5_pages_mb;

    for idx in 0..page_count {
        let bytes = src.page_bytes(idx)?;
        let t = Instant::now();
        let img = decode::decode(&bytes)?;
        decode_times.push(t.elapsed().as_secs_f64() * 1000.0);

        let t = Instant::now();
        let _resized = decode::resize_lanczos3(&img, 1920, 1080);
        resize_times.push(t.elapsed().as_secs_f64() * 1000.0);

        if idx % 25 == 0 {
            sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
            if let Some(rss) = sys
                .process(pid)
                .map(|p| p.memory() as f64 / (1024.0 * 1024.0))
            {
                peak_rss_mb = peak_rss_mb.max(rss);
            }
        }
    }
    decode_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    resize_times.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // ---- Phase E: idle-CPU window (5 seconds with nothing happening) ----
    sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
    let _prime = sys.process(pid).map(|p| p.cpu_usage()).unwrap_or(0.0);
    std::thread::sleep(Duration::from_secs(5));
    sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
    let idle_cpu_pct = sys
        .process(pid)
        .map(|p| p.cpu_usage() as f64)
        .unwrap_or(0.0);
    eprintln!("[spike] idle CPU over 5s: {:.2}%", idle_cpu_pct);

    // ---- Build report ----
    let report = BenchReport {
        version: env!("CARGO_PKG_VERSION").to_string(),
        platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        scenario: "phase1_cpu_spike".to_string(),
        fixture: FixtureInfo {
            path: fixture_path.display().to_string(),
            size_bytes: fixture_size,
            page_count,
        },
        timings: Timings {
            zip_open_ms,
            first_page_decode_ms,
            startup_to_first_page_ms,
            cold_decode_p50_ms: percentile(&decode_times, 50.0),
            cold_decode_p95_ms: percentile(&decode_times, 95.0),
            cold_decode_p99_ms: percentile(&decode_times, 99.0),
            cold_decode_max_ms: decode_times.last().copied().unwrap_or(0.0),
            resize_lanczos_p50_ms: percentile(&resize_times, 50.0),
            resize_lanczos_p99_ms: percentile(&resize_times, 99.0),
            parallel_preload_5_pages_ms,
            samples: page_count,
        },
        resource: ResourceUsage {
            idle_cpu_pct,
            working_set_after_5_pages_mb,
            peak_rss_mb,
        },
    };

    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
