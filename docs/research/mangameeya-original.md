# Original マンガミーヤ — Research Notes

## What it was

Windows-only manga reader, widely beloved in Japanese / CJK manga community for **speed** and **keyboard-driven UX**. Last known active development paused; current Windows community still uses it (2025 5ch threads active).

Variants:
- **MangaMeeya** — desktop Windows
- **MangaMeeyaCE** — Windows Mobile / WinCE (smaller subset)

## Why it's worth reviving

- Subjective page-turn latency that newer readers (e.g. CDisplayEx, Honeyview, MangaMeeya-clones) haven't matched
- Plug-in ecosystem (Susie .spi) means it can handle obscure archive / image formats
- Keyboard binding fully customisable via INI
- Single-folder install, fully portable
- **Unmaintained since ~2010 — known Unicode issues** (Shift-JIS assumption) and **32-bit memory ceiling** are not patchable by community; only a rewrite fixes them

---

## Technical architecture (deep dive, 2026-05)

Reverse-engineered from binary layout in [leopck/MangaMeeya/original](https://github.com/leopck/MangaMeeya/tree/master/original).

### Executable shape

```
MangaMeeya.exe   ~1.15 MB, 32-bit Windows native, x86
MangaMeeya-JP.exe ~1.13 MB, Japanese resource variant
```

Sub-megabyte main binary because the heavy lifting is in DLLs.

### Module layout

```
MangaMeeya.exe                          ← main app
│
├── arc.dll      (229 KB)              ← archive handler (zip/rar/lzh)
│
├── format decoders (separate DLLs, loaded on demand):
│   ├── jpg.dll  (782 KB)              ← libjpeg-derived
│   ├── png.dll  (217 KB)              ← libpng-derived
│   ├── jp2.dll  (811 KB)              ← OpenJPEG (JPEG 2000)
│   ├── gif.dll  (122 KB)
│   └── pdf.dll  (528 KB)              ← xpdf-based (xpdfrc config file)
│
├── AvisynthPlugin/                    ← image-processing pipeline (key insight!)
│   ├── MangaMeeya.dll      (294 KB)   ← MangaMeeya's own Avisynth filter
│   ├── lanczos3.dll        (94 KB)    ← Lanczos3 resize
│   ├── warpsharp.dll       (188 KB)   ← line-art sharpening (manga-specific!)
│   ├── waveletReducer.dll  (77 KB)    ← wavelet noise reduction
│   ├── SimpleResize.dll    (65 KB)    ← fast resize
│   ├── _2DCleanYUY2.dll               ← 2D denoising
│   ├── AdjustColor.dll                ← colour adjustment
│   ├── ColorYUY2.dll                  ← YUY2 colour-space conversion
│   └── ColorYUY2_for_25.dll
│
└── SusiePlugin/                        ← third-party format expansion
    └── ifjpeg.spi, ifpng.spi, axzip.spi, axpsd.spi, ax7z.spi, ...
```

Also:
- `page.wav` — page-turn sound effect
- `BookRackTexture.bmp`, `hondana*.jpg` — bookshelf UI assets
- `ToolButton*.bmp` — toolbar icons
- `Bookmarks.lst`, `Hist.lst`, `Playlist.lst` — persisted state

### The "fast" secret: Avisynth as image-processing engine

This is the **non-obvious architectural insight** about MangaMeeya. It does not roll its own image-filter code. It treats each page as a 1-frame video and runs it through [Avisynth](http://avisynth.nl/), which is a long-established video-frame processing framework with two decades of accumulated MMX/SSE hand-tuned assembly from the encoder community.

Consequence:

1. **Filter chain is config-driven**, defined in `MangaMeeya.ini`'s `[AvisynthPlugin]` section. 20+ predefined profiles (`ProfileName1` … `ProfileName21`) covering different manga page types (line art, screentone, colour, scanned print, etc.).
2. **Each filter is a separate DLL**. Users can add third-party Avisynth filters (an entire ecosystem exists).
3. **Filter chain looks like**:
   ```
   Source → Crop → Rotate → Lanczos3Resize → WarpSharp → ColorAdjust → Display
   ```
4. **Speed comes "for free"** by inheriting Avisynth filter optimisation. MangaMeeya itself does not need to write SIMD code.

Filters exposed by `MangaMeeya.dll` (Avisynth plug-in) per the `.def`:
- Clipping, Crop, Rotation, Resizing, Aspect-ratio resizing
- Page Display (multi-page rendering), Copy Rectangle
- Tone Curve (5-point bezier), Palette Reduction (2–256 colours with dithering)
- Anti-aliasing Smoothing, Colour-space Conversion (YUY2/YV12/RGB24/32/grey)
- Vignetting, Lens Distortion, Histogram Equalisation

### Cache strategy (from `MangaMeeya.ini`)

```ini
PrepageForwardNum=2       ; preload next 2 pages
PrepageBackwardNum=2      ; preload prev 2 pages
PrepageThreadPriority=2   ; dedicated preload thread, priority 2
PrepageFileCacheRate=75   ; 75% of cache reserved for preload
CacheNum=200              ; decoded RGBA cache: 200 entries
FileCacheSize=200         ; raw-bytes cache: 200 entries
GCLimitSize=300           ; threshold to trigger eviction
ResizeCache=1             ; cache resized output too
```

Three-layer cache: file bytes → decoded RGBA → resized output. All three cached, total ~200 entries. **Much more aggressive than what current Phase 1a plan assumes (5-page GPU texture cache).** Note we are talking about CPU-side RAM; GPU VRAM cache is a separate concern.

### Threading model (inferred)

At least two threads:
- **Main / UI thread** — winit-equivalent loop, decode, filter chain, blit
- **Preload thread** — `PrepageThreadPriority=2`, walks ahead/behind to fill cache

Avisynth itself can be multi-threaded but MangaMeeya's runtime usage of that is uncertain. leopck's note suggests filter chain runs synchronously on demand.

### Rendering

- **GDI / GDI+** for final framebuffer blit (no DX12/OpenGL involvement).
- Fullscreen mode is available (`FullScreen=1`, `Exclusive=1`).
- Smooth-scroll uses `ScrollSpool=1` and configurable speed (`SmoothScroll`).
- **No GPU acceleration.** All resize / filter is CPU-bound (CPU SIMD).

### Configuration system

- File: `MangaMeeya.ini` (~13.7 KB)
- Encoding: Shift-JIS (mojibake-prone for non-JP filenames — this is the Unicode problem)
- Sections include `[AvisynthPlugin]`, `[Keyboard]`, plus general settings
- `-i "path/to/your.ini"` CLI flag to switch profile

---

## Known limitations (confirmed)

| Issue | Root cause |
|---|---|
| Large zip crashes | 32-bit virtual address space + load-everything strategy |
| Multi-threading limited | Preload thread exists; decode + filter likely main-thread |
| SIMD only MMX/SSE | Avisynth filters are 1990s-2000s code; no AVX2/AVX-512 |
| **Unicode filenames break / mojibake** | Shift-JIS internal assumption, can't be patched without rewrite |
| 32-bit Susie plug-in lock-in | Existing .spi collection unusable from 64-bit host |
| No GPU acceleration | Pre-DirectX-era design; bottleneck on 4K/8K pages |
| No HiDPI awareness | Bitmap blit assumes 96 DPI |
| Discontinued | No fixes possible — only fork or rewrite |

---

## Sources

- [leopck/MangaMeeya — file inventory of original distribution](https://github.com/leopck/MangaMeeya/tree/master/original)
- [leopck/MangaMeeya — MangaMeeya.ini reference config](https://github.com/leopck/MangaMeeya/blob/master/original/MangaMeeya.ini)
- [leopck/MangaMeeya — Avisynth plug-in `.def` exports](https://github.com/leopck/MangaMeeya/blob/master/original/AvisynthPlugin/MangaMeeya.def)
- [Non-official atwiki (jp)](https://w.atwiki.jp/mangameeya) — comprehensive but 403 to automated fetchers
- [5ch software thread #24 (2025)](https://egg.5ch.net/test/read.cgi/software/1742548602/3-n) — ongoing user community
- [5ch software thread #18 (2018)](https://egg.5ch.net/test/read.cgi/software/1515748480/67-n)
- [マンガミーヤ プラグイン解説 (jp blog)](http://ykarenpgtm.dip.jp/link120.html)
- [Vector — RAR extract plug-in 詳細](https://www.vector.co.jp/soft/win95/art/se261123.html)
- [Avisynth project](http://avisynth.nl/)

## Unknowns / TODO

- [ ] Full default keymap list (atwiki block needs different fetch path)
- [ ] Exact INI section names and key formats (we have the file but haven't fully indexed)
- [ ] How Avisynth runtime is loaded — bundled or expects system install? (Probably bundled — `MangaMeeya.dll` in AvisynthPlugin/ is the host filter)
- [ ] Whether each filter profile is hot-switchable or requires reload
- [ ] Decoder-thread vs filter-chain-thread split — is filter on UI thread or worker?
- [ ] xpdf PDF rendering: synchronous? async? cached after first decode?

---

## Relation to this project

- We do **not** aim for bit-perfect compat. We aim for **subjective speed parity** and **keymap-shape parity** (default bindings feel familiar).
- **Filter chain is a real architectural insight** — our plug-in framework should expose filter plug-ins, not just format / archive plug-ins. See [`architecture.md`](../architecture.md).
- **Unicode is a free win** — Rust's UTF-8 + correct `OsString` handling solves the Shift-JIS pain with zero special-case code. This is the highest-leverage benefit of starting over.
- **GPU acceleration is a multiplier**, not a requirement. The original ran fine on GDI + CPU. Our wgpu pipeline can be massively faster on modern hardware, but a software-renderer fallback should still beat the original on AVX2 CPUs (since fast_image_resize uses AVX2 vs MangaMeeya's SSE-era code).
- **Cache budget should be more aggressive than current Phase 1a plan.** Increase decoded-bytes cache to 50-100 entries (CPU RAM, plenty available on 64-bit). GPU texture cache stays small (~5-10 entries, VRAM is the bottleneck).
- **Susie shim in Phase 4** is the migration carrot — anyone with .spi collection should not feel they're abandoning it.
- **32-bit Susie .spi on 64-bit host** is a known headache; Phase 4 will need to evaluate either (a) ship a 32-bit susie-host subprocess that pipes data, or (b) only support 64-bit SF-spec plug-ins. Decision deferred.
