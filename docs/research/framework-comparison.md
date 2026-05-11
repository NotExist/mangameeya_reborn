# GUI Framework Comparison

## Constraints applied

- Rust native (no FFI to GTK / Qt unless single-binary cost is acceptable — it isn't)
- **No WebView dependency** (rules out Tauri, dioxus-desktop in webview mode, Electron, Wails)
- **No system runtime dependency** (rules out gtk-rs, cxx-qt, qmetaobject)
- Single-folder portable output
- CJK / IME must be first-class
- Heavy image rendering + keyboard-driven UX
- Retained-mode preferred (reader is mostly idle on a single page)

## Survivors of constraint filtering

Only three Rust-native GUI frameworks pass: **Iced**, **Slint**, **egui**.

## Detailed comparison

| Dimension | Iced 0.14+ | Slint 1.x | egui |
|---|---|---|---|
| Render backend | wgpu (default) + tiny-skia fallback | Skia / FemtoVG / Software / wgpu (1.12+) | wgpu / glow |
| GPU shader integration | `widget::shader` API (mature) | wgpu embed (1.12+, newer) | `egui_wgpu::CallbackTrait` |
| Mode | Retained (Elm) | Retained (declarative DSL) | Immediate |
| Idle CPU | ≈ 0% | ≈ 0% | needs `request_repaint` discipline |
| Keyboard / focus | 0.14 IME improved, mature dispatch | Best — IME report most polished | Known IME bugs (eats Tab during composition) |
| CJK font rendering | OK with manual font loading | Best — fontdb integrated | OK with `egui-cjk-font` |
| HiDPI | Yes | Yes | Yes |
| API stability | 0.x, breaking changes between versions | 1.x stable API commitment | Stable-ish, occasional API churn |
| Bundle size | Pure Rust binary, ~5-15MB | Pure Rust binary, ~5-15MB | Pure Rust binary, ~5-10MB |
| Real image-viewer precedent | **ViewSkater** (Iced + wgpu, 8K images) | Industrial / HMI use, few image viewers | **aw-man (early)**, **avis-imgv** |
| Licensing | MIT | GPL or commercial / royalty-free tiers | MIT |
| Layout flexibility | Good, less than HTML/CSS | DSL is powerful for shelf grids | Minimal — manual layout |
| Bookshelf virtualisation | Manual `scrollable` + culling | Built-in `ListView` (lazy) | `ScrollArea::show_rows` |

## Decision

**Primary: Iced**

Reasons:
1. [ViewSkater](https://viewskater.com/) is the **closest real-world precedent** to our use case — Iced + wgpu, GPU texture cache, 8K images. The dangerous part of the stack is proven.
2. `widget::shader` lets us own the reader view as a wgpu canvas while still using Iced for shelf / settings / dialogs. **No framework overhead on the hot path.**
3. Elm architecture maps cleanly to "current page / reading direction / cache / shelf" — discrete state, clear messages.
4. MIT license — no future commercial / openness concerns.

**Fallback: Slint**

Trigger: Iced's CJK / IME performance fails Phase 2 acceptance.
Reason Slint is the fallback rather than primary: less production data on image-heavy desktop apps, licensing complexity (commercial tier exists), and the DSL has a learning curve that adds calendar time on day 1.

**Rejected (with rationale)**

- **egui**: immediate mode is wrong for an app that's idle 95% of the time; IME bugs are a target-audience deal-breaker.
- **gtk-rs / relm4**: needs GTK runtime DLLs on Win/Mac — fails portable goal.
- **cxx-qt**: best technical option in absolute terms, but Qt runtime + licensing complexity not worth it for a small project.
- **floem / xilem / blitz / makepad**: pre-1.0, accessibility / IME / font work incomplete.
- **fltk-rs**: layout system too weak for shelf / settings UIs.

## Hot-path architecture (regardless of framework)

Reader view is **always** a `wgpu`-managed canvas embedded as a `widget::shader` (Iced) or `wgpu_28` integration (Slint). The framework owns chrome (menus, dialogs, shelf), but the actual image rendering is bare-metal wgpu. This isolates "framework choice" risk from "rendering speed" — switching frameworks would barely touch the hot path.

## Sources

- [boringcactus — 2025 Survey of Rust GUI Libraries](https://www.boringcactus.com/2025/04/13/2025-survey-of-rust-gui-libraries.html)
- [ViewSkater](https://viewskater.com/) — Iced + wgpu image viewer
- [ViewSkater dev blog](https://ggando.com/blog/imageviewer0/)
- [aw-man](https://github.com/awused/aw-man) — Rust GTK4 manga reader (different stack, similar problem)
- [Iced widget::shader docs](https://docs.rs/iced/latest/iced/widget/shader/index.html)
- [Iced 0.14 release HN discussion](https://news.ycombinator.com/item?id=46185323)
- [Slint 1.12 wgpu integration announcement](https://slint.dev/blog/slint-1.12-released)
- [Slint Making Desktop Ready](https://slint.dev/blog/making-slint-desktop-ready)
- [egui CJK / IME tab-eat bug (#3060)](https://github.com/emilk/egui/issues/3060)
- [Tauri vs Iced vs egui performance comparison](http://lukaskalbertodt.github.io/2023/02/03/tauri-iced-egui-performance-comparison.html)
