# MangaMeeya Reborn

用現代技術重現 Windows 上已年久失修的漫畫閱讀器 **マンガミーヤ (MangaMeeya)**。

## Status

**Phase 0 — Planning complete. Phase 1 (Speed Spike) pending start.**

無實際程式碼，目前為文件與架構規劃階段。

## Goals

- **速度與鍵盤體感至上** — 比照原版マンガミーヤ的低延遲翻頁與全鍵盤操作
- **原生桌面，不依賴外部 runtime** — 單資料夾 portable
- **三平台空間** — Windows-first，但架構為三平台預留
- **CJK / IME 為一等公民** — 目標族群是日漢圈使用者
- **插件式格式支援** — WASM-first，相容 Susie plug-in (Windows 限定)
- **接通 manga server 生態** — OPDS client，未來可串 Suwayomi / Komga / Kavita

## Reading order

1. [docs/plan.md](docs/plan.md) — 分階段執行計劃 (Phase 1 ~ 6)
2. [docs/architecture.md](docs/architecture.md) — 模組分層、`PageSource` trait、插件框架
3. [docs/research/](docs/research/) — 原版分析、Susie 規格、框架對比、生態系研究
4. [docs/test-checklist.md](docs/test-checklist.md) — 實體機器測試清單（Phase 1b / Phase 2 需在你機器跑）
5. [docs/backlog.md](docs/backlog.md) — 已知待改進事項

## Bench fixture

CPU spike benchmark expects a manga-shaped zip. Two ways to provide it:

1. **Real fixture (preferred)** — set repo variable `BENCH_FIXTURE_URL` (Settings → Secrets and variables → Actions → Variables) to a URL hosting an actual zip. The Bench workflow fetches it on each run. Keeps potentially copyrighted content out of the repo.
2. **Synthetic fallback** — if `BENCH_FIXTURE_URL` is unset, `gen_fixture` synthesises 250 pages of procedural noise. Compression ratio is poor on noise so the synthetic zip is ~1.3GB rather than the ~400MB a real manga would produce; decode/resize timings still hold relative meaning.

## Non-goals

- 線上漫畫源（將透過 OPDS 串接既有 server，不重造 Mihon 生態）
- 行動裝置（不在規劃中）
- WebView / Electron / Tauri 路線（明確排除）
- 編輯/管理書庫的複雜介面（最近開過 + 即時縮圖即可）

## License

未定。當前自用、未來可能開源、不轉商用。授權決定推遲到首次公開發佈前。
