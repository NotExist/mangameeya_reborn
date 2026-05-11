# Architecture

## Module layering

```
┌────────────────────────────────────────────────────────────┐
│  mm-ui  (Iced — 唯一 import iced 的 crate)                 │
│  ├─ ReaderView    雙頁/單頁、右翻、wgpu canvas             │
│  ├─ ShelfView     最近開過、即時縮圖                       │
│  ├─ KeymapEditor                                           │
│  ├─ SettingsView                                           │
│  └─ DialogHost    開檔、訊息                               │
├────────────────────────────────────────────────────────────┤
│  Framework-agnostic core (任何 GUI 都能換上)               │
│  ├─ mm-core       PageSource trait、共用型別               │
│  ├─ mm-source     LocalFolder / Zip                        │
│  ├─ mm-plugin-host wasmtime 載入器                         │
│  ├─ mm-plugin-sdk plug-in 作者用的 Rust crate              │
│  ├─ mm-decode     rayon pool + GPU texture cache           │
│  ├─ mm-keymap     Action + binding 持久化                  │
│  ├─ mm-config     TOML 設定                                │
│  └─ mm-recent     最近開過 JSON                            │
├────────────────────────────────────────────────────────────┤
│  Platform                                                   │
│  ├─ wgpu          自訂 Lanczos shader                      │
│  ├─ winit         視窗、輸入                               │
│  ├─ rayon         CPU-bound worker (decode/resize)         │
│  └─ tokio         I/O-bound worker (Phase 6 remote)        │
└────────────────────────────────────────────────────────────┘
```

**Hard rule:** `mm-ui` 是唯一能 `use iced` 的 crate。其他 crate 對 UI 框架完全無感。CJK / IME 在 Phase 2 落敗時換 Slint，只動 `mm-ui` 一個 crate。

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

## Image pipeline

```
Archive entry (raw bytes)
        ↓ [worker: rayon]
image::load_from_memory()       ← supports JPEG/PNG/WebP/AVIF/BMP/GIF/TIFF
        ↓
image::DynamicImage (CPU)
        ↓ [worker: rayon]
image::imageops::resize(Lanczos3)  ← CPU pre-resize to target dimensions
        ↓
RGBA bytes
        ↓ [main: wgpu queue.write_texture]
wgpu::Texture (GPU)
        ↓ [main: render pass]
Fragment shader: Lanczos / Mitchell / Bilinear (user pick)
        ↓
Backbuffer → Present
```

### Cache layers

| Cache | Storage | Eviction | Size |
|---|---|---|---|
| Page bytes (compressed) | RAM | LRU | ~50 entries |
| Decoded RGBA (CPU) | RAM | LRU | ~10 entries |
| GPU texture | VRAM | LRU | ~5 entries |
| Thumbnails | RAM | LRU | ~200 entries (small) |

預讀策略：根據 reading direction 與閱讀速度，預測下 N±2 頁，丟 worker pool 預先走過 pipeline 到 GPU texture stage。

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
filter = "lanczos"          # lanczos | mitchell | bilinear | nearest
preload_pages = 2

[ui]
theme = "dark"              # dark | light | system
font_family = ""            # empty = system default
font_size_ui = 14
font_fallback_cjk = "Noto Sans CJK"

[performance]
gpu_texture_cache = 5
cpu_decode_cache = 10
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
