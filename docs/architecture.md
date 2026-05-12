# Architecture

## Module layering

```
┌────────────────────────────────────────────────────────────┐
│  mm-ui  (Iced — 唯一 import iced 的 crate)                 │
│  ├─ ReaderView    雙頁/單頁、右翻                          │
│  ├─ ShelfView     最近開過、即時縮圖                       │
│  ├─ KeymapEditor                                           │
│  ├─ SettingsView                                           │
│  └─ DialogHost    開檔、訊息                               │
├────────────────────────────────────────────────────────────┤
│  mm-render (trait PageRenderer — 多 backend 抽象)           │
│  ├─ WgpuRenderer    wgpu + custom shader (default, Win10+) │
│  └─ SoftwareRenderer softbuffer/tiny-skia (legacy, Win7+)   │
├────────────────────────────────────────────────────────────┤
│  Framework-agnostic core (任何 GUI / 任何 renderer 都能用)  │
│  ├─ mm-core       PageSource trait、共用型別               │
│  ├─ mm-source     LocalFolder / Zip                        │
│  ├─ mm-plugin-host wasmtime 載入器                         │
│  ├─ mm-plugin-sdk plug-in 作者用的 Rust crate              │
│  ├─ mm-decode     rayon pool + decoded-bytes cache         │
│  ├─ mm-filter     filter chain (Phase 4, 受 Avisynth 啟發) │
│  ├─ mm-keymap     Action + binding 持久化                  │
│  ├─ mm-config     TOML 設定                                │
│  └─ mm-recent     最近開過 JSON                            │
├────────────────────────────────────────────────────────────┤
│  Platform                                                   │
│  ├─ wgpu (默認)    自訂 Lanczos shader                     │
│  ├─ softbuffer (legacy) RGBA blit                          │
│  ├─ winit         視窗、輸入                               │
│  ├─ rayon         CPU-bound worker (decode/resize)         │
│  └─ tokio         I/O-bound worker (Phase 6 remote)        │
└────────────────────────────────────────────────────────────┘
```

**Hard rules**

1. **`mm-ui` 是唯一能 `use iced` 的 crate。** 其他 crate 對 UI 框架完全無感。CJK / IME 在 Phase 2 落敗時換 Slint，只動 `mm-ui` 一個 crate。
2. **`mm-render` 是唯一能 `use wgpu` 的 crate（除非透過 `gpu-spike` feature）。** 其他 crate 透過 `PageRenderer` trait 操作。Phase 5+ 的 legacy-windows build 換成 software renderer 時，只動 `mm-render` 一個 crate。
3. **核心 crate（`mm-core` 以下到 `mm-filter`）對平台、UI 框架、renderer 後端完全無感**——只依賴 std 與純 Rust 第三方 crate。任何 platform conditional 都被推到上層 crate。

## Render backend abstraction (PageRenderer trait)

受「我自己是 Win7 使用者」需求與「保留 legacy Windows 路徑」設計目標驅動，渲染層走 trait 抽象，預設 wgpu，可換軟體實作。

```rust
pub trait PageRenderer: Send {
    /// 把解碼後的圖像上載到 backend（GPU texture 或 CPU framebuffer）
    fn upload_page(&mut self, image: &DynamicImage) -> Result<()>;

    /// 觸發一幀畫面輸出
    fn render(&self) -> Result<()>;

    /// 視窗大小變更
    fn resize(&mut self, width: u32, height: u32);

    /// Backend 自報能力（影響 UI 是否顯示某些選項）
    fn capabilities(&self) -> RendererCapabilities;
}

pub struct RendererCapabilities {
    pub has_gpu_accel: bool,
    pub max_texture_dim: u32,
    pub supports_custom_shader: bool,
}
```

### Backend 對照

| 維度 | `WgpuRenderer` (default) | `SoftwareRenderer` (legacy) |
|---|---|---|
| 後端 | wgpu (DX12 / Vulkan / Metal) | softbuffer + tiny-skia |
| 平台 | Win10+, macOS 10.13+, Linux Vulkan/X11 | Win7+, macOS 10.7+, Linux X11/Wayland |
| Lanczos 縮放 | GPU fragment shader | CPU via `fast_image_resize` (AVX2) |
| 自訂 shader | ✓ | ✗（hard-coded blit） |
| HiDPI | 完整 | 基本（不像 wgpu 那樣精準） |
| 啟動時間 | ~50ms (GPU init) | <5ms |
| 翻頁 latency | < 1ms (texture binding swap) | ~10-20ms (RGBA blit) |
| 4K/8K 大圖效能 | 極佳 | 可接受 (CPU SIMD) |
| 風險 | 需 GPU driver | 老 driver 也能跑 |

Phase 1b 完成的 spike_gpu 就是 WgpuRenderer 的雛形。`SoftwareRenderer` 是 Phase 5+ 才實作的 deliverable。

### Cargo feature gating

```toml
[features]
default = ["render-wgpu"]
render-wgpu = ["dep:wgpu"]
render-software = ["dep:softbuffer", "dep:tiny-skia"]
legacy-windows = ["render-software"]   # convenience meta-feature
```

`cargo build --release` → 預設 wgpu。
`cargo build --release --no-default-features --features render-software` → 軟體渲染版。
`cargo build --release --no-default-features --features legacy-windows --target x86_64-win7-windows-msvc` → Win7 build。

## Core trait — PageSource

## Core trait — PageSource

```rust
#[async_trait]
pub trait PageSource: Send + Sync {
    /// Source 顯示用 metadata（標題、頁數、封面 hint）
    async fn metadata(&self) -> Result<SourceMeta>;

    /// 總頁數。對於串流型 source（如 OPDS）可能是初始估計
    async fn page_count(&self) -> Result<usize>;

    /// 取得指定頁的原始 bytes（尚未解碼）
    async fn page_bytes(&self, idx: usize) -> Result<Bytes>;

    /// 取得縮圖。實作可以選擇從原圖縮、或 source 端有預生縮圖就直接拿
    async fn thumbnail(&self, idx: usize, size: ThumbnailSize) -> Result<Bytes>;
}
```

實作清單：

| Impl | Phase | 說明 |
|---|---|---|
| `LocalFolderSource` | 3 | 一個資料夾內的圖像，自然排序 |
| `ZipSource` | 3 | 內建 zip/cbz 讀取，seek-based、不展開到磁碟 |
| `PluginArchiveSource` | 4 | 透過 WASM plug-in 處理 RAR/7z/PDF 等 |
| `SusiePluginSource` | 4 | 透過 Susie .spi shim（Windows 限定） |
| `RemoteOpdsSource` | 6 | OPDS catalog + HTTP fetch |

本地 source 的 `async` 只是無消費（內部走 `spawn_blocking` 包同步 IO）；遠端 source 就吃 reqwest。**對 reader 完全透明。**

## Plug-in framework

### 三層 plug-in 來源

```
PluginHost
├─ Built-in        zip, cbz, jpg, png, webp, avif, bmp, gif
├─ WasmPlugin      *.mmplug (WASM + manifest.toml)
└─ SusieShim       *.spi (Windows only, Phase 4)
```

### Manifest 結構（`manifest.toml`）

```toml
[plugin]
id = "rar-archive"
name = "RAR Archive Support"
version = "0.1.0"
author = "..."
mm_api = "1"

[capabilities]
kind = "archive"            # archive | image | source
formats = ["rar", "cbr"]
mime_types = ["application/x-rar-compressed"]

[wasm]
file = "rar_archive.wasm"
```

### WASM ABI（高層意圖，不是最終定義）

```rust
// Plug-in author writes Rust → compile to wasm32-wasi
#[mangameeya_plugin::plugin]
impl ArchivePlugin for RarPlugin {
    fn supported_extensions() -> &'static [&'static str] {
        &["rar", "cbr"]
    }

    fn open(&self, bytes: &[u8]) -> Result<Vec<EntryInfo>> { ... }

    fn read_entry(&self, idx: usize) -> Result<Vec<u8>> { ... }
}
```

### Why WASM not native DLL

| 維度 | WASM | Native DLL |
|---|---|---|
| 跨平台 | 同一個 binary 跑 Win/Mac/Linux | 三份 binary |
| 安全沙箱 | 預設無檔系統存取 | 完全信任 |
| 效能 | -10~20% (insignificant for I/O bound) | baseline |
| 工具鏈 | wasmtime 一個 Rust dep | OS loader |
| 版本問題 | 嚴格 ABI via manifest | DLL hell |

### Why still support Susie

Windows 老玩家手上累積了大量 .spi。Phase 4 提供 shim 是「遷移路徑」，不是核心功能。Linux/macOS 不支援是預期內。

## Filter chain (Phase 4 — 受 Avisynth pipeline 啟發)

原版 MangaMeeya 把 Avisynth 視訊框架當圖像 pipeline 用，配置 20+ 個 filter profile 因應不同 manga 類型（黑白原稿 vs 彩頁 vs 印刷掃描）。詳見 [research/mangameeya-original.md](research/mangameeya-original.md#technical-architecture-deep-dive-2026-05)。

我們 Phase 4 引入對應的設計：**filter chain plug-in**。

### Plug-in kind 擴充

```toml
# 既有
kind = "archive"   # 壓縮檔處理
kind = "image"     # 圖像 decoder
kind = "source"    # 遠端 / 本地 source

# 新增 (Phase 4)
kind = "filter"    # 圖像 post-decode 處理（resize / sharpen / colour）
```

### Filter trait（WASM 端）

```rust
#[mangameeya_plugin::plugin]
impl ImageFilter for WarpSharpFilter {
    fn name() -> &'static str { "warpsharp" }

    fn parameters() -> &'static [FilterParam] {
        &[
            FilterParam::float("threshold", 0.0, 1.0, default: 0.5),
            FilterParam::int("range", 0, 8, default: 2),
        ]
    }

    fn apply(&self, image: &mut FilterImage, params: &FilterParams) -> Result<()> {
        // RGBA8 in-place mutation; image: &mut [u8] with width/height
    }
}
```

### Filter profile（使用者層）

```toml
# mangameeya.toml
[[filter_profile]]
name = "Manga (line art)"
chain = [
  { filter = "lanczos3_resize", scale = "fit_window" },
  { filter = "warpsharp", threshold = 0.4 },
]

[[filter_profile]]
name = "Manga (screentone)"
chain = [
  { filter = "lanczos3_resize", scale = "fit_window" },
  { filter = "wavelet_denoise", strength = 0.3 },
]

[[filter_profile]]
name = "Color page"
chain = [
  { filter = "lanczos3_resize", scale = "fit_window" },
  { filter = "color_adjust", contrast = 1.05, gamma = 1.0 },
]
```

切換 filter profile 是 keyboard shortcut，符合「全鍵盤操作」的設計目標。

### Hot-path 優化

- **熱路徑 (resize)** 不走 WASM —— 直接 `fast_image_resize` (CPU AVX2) 或 wgpu fragment shader (GPU)，跳過 plug-in overhead
- **配色 / 銳化 / 去噪 等**走 WASM filter plug-in（每張頁面執行一次，10-20% WASM overhead 可接受）
- **濾鏡可組合**但 chain 太長時自動警告（性能負面）

### Built-in filters (Phase 4)

- `lanczos3_resize`、`mitchell_resize`、`bilinear_resize`、`nearest_resize`
- `warpsharp` (移植自 Avisynth `warpsharp.dll`)
- `unsharp_mask`
- `color_adjust` (gamma / brightness / contrast / saturation)
- `wavelet_denoise`
- `rotate_90` / `rotate_180` / `rotate_270` / `flip_h` / `flip_v`
- `crop_borders` (自動偵測掃描白邊)

第三方 filter 透過 WASM plug-in 安裝，與 archive / source plug-in 同走 `plugins/` 目錄。

## Image pipeline

```
Archive entry (raw bytes)
        ↓ [worker: rayon]
image::load_from_memory()       ← supports JPEG/PNG/WebP/AVIF/BMP/GIF/TIFF
        ↓
image::DynamicImage (CPU)
        ↓ [worker: rayon]
fast_image_resize::resize(Lanczos3)  ← CPU AVX2-accelerated resize
        ↓
[optional: filter chain (Phase 4) — sharpen / colour / denoise]
        ↓
RGBA bytes
        ↓ [PageRenderer::upload_page]
        │
        ├─ WgpuRenderer:    queue.write_texture → fragment shader → present
        └─ SoftwareRenderer: softbuffer buffer  → tiny-skia blit  → present
```

### Cache layers

| Cache | Storage | Eviction | Size | Note |
|---|---|---|---|---|
| Page bytes (compressed) | RAM | LRU | ~50 entries | 4GB+ 系統可放更多 |
| Decoded RGBA (CPU) | RAM | LRU | ~50 entries | 比 Phase 1a 原規劃激進（受原版 200 entries 啟發） |
| GPU texture (wgpu only) | VRAM | LRU | ~5 entries | VRAM 寶貴，保守即可 |
| Software framebuffer | RAM | 1 current | 1 | legacy backend 不需多重 |
| Thumbnails | RAM | LRU | ~200 entries (small) | |

設定可調，default 上面數字。`mangameeya.toml [performance]` 段使用者可改。

預讀策略：根據 reading direction 與閱讀速度，預測下 N±2 頁，丟 worker pool 預先走過 pipeline 到 PageRenderer 的 upload stage。

## Keymap

```toml
# keymap.toml
[bindings]
"Right"           = "next_page"
"Left"            = "prev_page"
"Space"           = "next_page"
"Shift+Space"     = "prev_page"
"Home"            = "first_page"
"End"             = "last_page"
"F11"             = "toggle_fullscreen"
"Ctrl+O"          = "open_file"
"Ctrl+,"          = "open_settings"
",,"              = "open_recent"        # chord
"gg"              = "first_page"         # vim-like chord
"G"               = "last_page"
```

`Action` 是個 enum，dispatcher 在 reader hot path 直接 match，無動態查找成本。

Chord（多鍵序列）支援是 nice-to-have，Phase 3 內若時間允許就做；否則 Phase 4。

## Config schema

```toml
# mangameeya.toml
[reader]
default_mode = "dual_rtl"   # dual_rtl | dual_ltr | single
fit_mode = "fit_window"     # fit_window | fit_width | fit_height | actual
filter_profile = "default"  # 對應 [[filter_profile]] 段 (Phase 4)
preload_pages = 2

[ui]
theme = "dark"              # dark | light | system
font_family = ""            # empty = system default
font_size_ui = 14
font_fallback_cjk = "Noto Sans CJK"

[render]
backend = "auto"            # auto | wgpu | software
                            # auto 會偵測 GPU，失敗 fallback 到 software

[performance]
cpu_decode_cache = 50       # 解碼後 RGBA cache (RAM)
gpu_texture_cache = 5       # GPU texture cache (wgpu backend only)
page_bytes_cache = 50       # 壓縮 bytes cache
worker_threads = 0          # 0 = auto (num_cpus)

[paths]
plugins_dir = "./plugins"
susie_dir = "./susie"       # Windows only
cache_dir = "./cache"
```

## Portable layout (recap)

```
mangameeya/
├─ mangameeya.exe (Win) | mangameeya.app (mac) | mangameeya (Linux)
├─ mangameeya.toml
├─ keymap.toml
├─ recent.json
├─ plugins/
│  ├─ rar/
│  │  ├─ manifest.toml
│  │  └─ rar_archive.wasm
│  └─ ...
├─ susie/                   # Windows only
│  └─ *.spi
├─ cache/
│  ├─ thumbs/
│  └─ ...
└─ logs/
```

Override via `MANGAMEEYA_HOME=/some/path` 或 `-c /some/path/config.toml`。

## Threading model

| Thread | Job |
|---|---|
| Main (UI) | winit event loop, input, render submission |
| Rayon pool | decode, resize, thumbnail generation |
| Tokio (Phase 6) | HTTP fetch for OPDS |
| Plugin worker (Phase 4) | wasmtime instance pool (1 instance per plug-in) |

**Forbidden on main thread:** any `std::fs::read`、`zip::read::ZipArchive::by_index`、`image::open`、`image::imageops::resize`。一律 `tokio::task::spawn_blocking` 或 rayon。
