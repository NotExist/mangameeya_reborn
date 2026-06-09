# Hamana Reborn — Agent Guidance

> Project's irreducible conventions. The plan lives in `docs/plan.md`; architecture in `docs/architecture.md`. Read those first.

## Project intent

Build a **Hamana-style** (宮野牧人, 2006) native, single-folder-portable desktop image/manga viewer in Rust. The defining UI is Hamana's: floating auto-hide overlay (top thumbnails, bottom-right IMAGE SEEK BAR), maximum image area, GPU-accelerated zoom/pan with zero perceptible lag.

Archive format support (ZIP/CBZ/RAR/CBR direct read) and keyboard-first navigation are borrowed from **マンガミーヤ (mangameeya)**, also abandoned. mangameeya is treated only as a design reference here, not as a reimplementation target.

Speed and keyboard ergonomics are non-negotiable. CJK / IME quality is non-negotiable.

## Hard architectural rules

1. **UI framework code is isolated.** Anything that imports `iced` / `slint` lives in `crates/ui/` (or equivalent). `core`, `plugin-host`, `source`, `decode`, `keymap`, `config` are framework-agnostic and must compile without the UI crate.
   *Reason:* CJK / IME testing in Phase 2 may force a switch from Iced to Slint. The cost of that switch must be a rewrite of one crate, not the whole app.

1a. **Render backend is isolated behind `PageRenderer` trait.** Only `crates/render/` (or equivalent) may `use wgpu`. Other crates operate through the trait. A `SoftwareRenderer` impl will be added in Phase 5 for legacy-Windows (Win7) builds via Cargo feature `legacy-windows`.
   *Reason:* The developer is a Win7 user; original MangaMeeya's Shift-JIS / Unicode issues are unpatchable. The rewrite must preserve a path to ship a working Win7 binary even if its renderer is downgraded to software. XP is not supported.

2. **No external runtime dependencies in the shipped artifact.** No GTK DLLs, no Qt, no system WebView, no VC++ redistributable. Static-link the MSVC CRT on Windows (`-C target-feature=+crt-static`). Acceptable: native OS graphics APIs (Vulkan / Metal / DX12) — those are the OS itself.

3. **Single-folder portable.** All config, cache, plugins, logs live under the application folder by default (relative to the exe). No `%APPDATA%`, no `XDG_*` writes unless the user opts in. Honour `HAMANA_HOME` env var as override.

4. **`PageSource` is async from day 1**, even if MVP only fills it with local sources. Phase 6 adds OPDS — the trait must not need a breaking change.

5. **Heavy work never blocks the UI thread.** Archive read, image decode, image resize all run on a worker pool (`rayon` for CPU-bound, `tokio` for network). Keyboard input → state mutation → `request_redraw()` is the only thing the UI thread does on the hot path.

6. **Plugins go through a sandbox boundary.** First-class plugin format is WASM (via `wasmtime`). Susie `.spi` compat is a Windows-only shim, last priority.

7. **CJK smoke test for any UI text change.** If a PR touches text rendering, font selection, IME-adjacent code, or path display, it must include screenshots / tests of: 日文檔名、中文檔名、IME 組字過程、缺字 fallback。

## Configuration

- **Format: TOML.** Not JSON, not YAML. Human-editable, supports comments, mature Rust tooling.
- **Location: `./hamana.toml`** next to the exe, overridable via `HAMANA_HOME` or `-c <path>` CLI flag.
- **Keymap is a separate file** (`./keymap.toml`) so power users can share bindings without leaking other settings.

## Testing discipline

- Core logic (plugin host, source, decode, keymap, config) must be 100% testable via `cargo test` on CI without a display.
- Image-pipeline output gets **screenshot regression tests** via wgpu offscreen render + baseline PNG diff.
- Real UI-driving e2e (clicking buttons via OS automation) is deferred — `enigo` + screenshot is fragile and GitHub Actions has no easy GPU runner. Treat e2e as nice-to-have, not blocker.

## Performance budget (Phase 1 acceptance criteria)

| Metric | Target |
|---|---|
| KeyDown → frame present | ≤ 16ms @ 60Hz, ≤ 8ms @ 120Hz |
| Pre-decoded page turn | < 1ms |
| Cold decode of 6000×4000 JPEG | < 200ms |
| 100-page zip → first page visible | < 500ms |
| Idle CPU | ≈ 0% |
| Memory with 5-page cache | < 500MB |

If a change regresses any of these by >10%, do not merge without a profile trace and an explanation.

## Commits

- Never commit `target/`, IDE files, OS metadata.
- Never skip git hooks (`--no-verify`) unless explicitly asked.
- Commit `Cargo.lock` (this is a binary, not a library).

## What this file is not

Not a plan, not architecture documentation, not a roadmap. Those live in `docs/`. This file is conventions that are easy to break by accident and hard to recover from. Update only when an irreducible rule changes.
