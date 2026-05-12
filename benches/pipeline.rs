use criterion::{Criterion, criterion_group, criterion_main};
use mangameeya_reborn::{decode, fixture};
use std::path::Path;
use std::sync::OnceLock;

fn page_normal() -> &'static [u8] {
    static CELL: OnceLock<Vec<u8>> = OnceLock::new();
    CELL.get_or_init(|| {
        let path = Path::new("bench-data/page_normal.jpg");
        if let Ok(b) = std::fs::read(path) {
            return b;
        }
        let bytes = fixture::synthesize_page(1, 2400, 3400);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, &bytes);
        bytes
    })
}

fn page_extreme() -> &'static [u8] {
    static CELL: OnceLock<Vec<u8>> = OnceLock::new();
    CELL.get_or_init(|| {
        let path = Path::new("bench-data/page_extreme.jpg");
        if let Ok(b) = std::fs::read(path) {
            return b;
        }
        let bytes = fixture::synthesize_page(1, 6000, 4000);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, &bytes);
        bytes
    })
}

fn bench_decode_normal(c: &mut Criterion) {
    let bytes = page_normal();
    c.bench_function("decode_2400x3400_jpeg", |b| {
        b.iter(|| decode::decode(bytes).unwrap())
    });
}

fn bench_decode_extreme(c: &mut Criterion) {
    let bytes = page_extreme();
    let mut g = c.benchmark_group("decode_6000x4000_jpeg");
    g.sample_size(20);
    g.bench_function("decode", |b| b.iter(|| decode::decode(bytes).unwrap()));
    g.finish();
}

fn bench_resize_lanczos(c: &mut Criterion) {
    let bytes = page_normal();
    let img = decode::decode(bytes).unwrap();
    let mut g = c.benchmark_group("resize_lanczos3");
    g.sample_size(30);
    g.bench_function("2400x3400_to_1080p", |b| {
        b.iter(|| decode::resize_lanczos3(&img, 1920, 1080).unwrap())
    });
    g.finish();
}

criterion_group!(
    benches,
    bench_decode_normal,
    bench_decode_extreme,
    bench_resize_lanczos
);
criterion_main!(benches);
