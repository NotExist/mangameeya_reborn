# Execution Plan

六階段計劃。每個 Phase 都有明確的「通過標準」，未達標不前進。

---

## Phase 0 — Closed ✓

Decisions locked:

| 議題 | 決定 |
|---|---|
| 形態 | 原生桌面 Rust 應用，不走 WebView |
| 主框架 | Iced (0.14+)，Slint 為 CJK/IME 落敗時的備援 |
| 平台 | Windows-first，三平台保留空間 |
| 發佈 | 自用 → 可能開源 → 不轉商用 |
| 設定格式 | TOML |
| 分發形態 | 單資料夾 portable |
| MVP 範圍 | 最近開過 + 即時縮圖（無書庫） |
| 格式支援 | 內建 zip/cbz/常見圖像；其餘走 WASM 插件 |
| 跨平台 plug-in | WASM (wasmtime) |
| Windows 相容路徑 | Susie `.spi` shim（Phase 4） |
| 線上來源 | OPDS client（Phase 6），非 Mihon extension 重造 |
| 網路掛載 | 透明（OS 處理），async I/O discipline |
| CI/CD | GitHub Actions matrix + Releases + Pages |

開放問題全數收口。

---

## Phase 1 — Speed Spike

**Duration:** 1–2 週
**Disposable:** 是。本 Phase 程式碼預期丟棄。

### 目的

在「**完全沒有 UI 框架**」的情況下，先證明速度天花板。確認 wgpu + winit + zip 解碼 + 圖像 pipeline 的端到端延遲達標。

### 範圍

- `winit` + `wgpu`，無 Iced / Slint
- 寫死讀取一個 `.zip` 檔（測試用，路徑硬編）
- 雙頁顯示、左右方向鍵翻頁
- 無 shelf、無設定、無美感、無錯誤處理

### 通過標準

| 指標 | 目標 | 量測方式 |
|---|---|---|
| KeyDown → frame present latency | ≤ 16ms (60Hz) | winit timestamp + wgpu present callback |
| 翻頁時 frame drop | 0 drops | wgpu submission queue 觀察 |
| 6000×4000 JPEG 冷解碼 | < 200ms | `std::time::Instant` |
| 預解碼後翻頁 | < 1ms | 同上 |
| 100 頁 zip 開檔→第一頁可見 | < 500ms | 同上 |
| Idle CPU | ≈ 0% | OS task manager |
| 5 頁 cache 記憶體 | < 500MB | OS task manager |

任一指標未達 → 不進 Phase 2，先檢視瓶頸（解碼？wgpu upload？archive seek？）。

---

## Phase 2 — Framework Selection + CJK Verification

**Duration:** 3–5 天

### 目的

把 Phase 1 的 wgpu canvas 嵌入 Iced 的 `widget::shader`，量測整合後仍達 Phase 1 標準；並全面驗證 CJK / IME 表現。Slint 作為影子分支同步建立，以備不時之需。

### 工作項

1. Iced 整合：`widget::shader` 包裝 wgpu canvas
2. Slint 影子分支：相同 wgpu 邏輯接到 Slint
3. CJK / IME 四項驗證（Windows 上跑，因為 Windows IME 是最大未知）：
   - 日文檔名顯示（含半形 / 全形混雜）
   - 中文檔名顯示（繁/簡）
   - 搜尋框 IME 組字過程不掉鍵、不偷 Tab
   - 字型 fallback（缺字不出 tofu）
4. HiDPI 銳利度驗證（125%、150%、200% 縮放）

### 通過標準

- Iced 整合後仍達 Phase 1 全部指標（容許 5% 退化）
- CJK / IME 四項全通過 → **定案 Iced**
- 若 Iced 任一項顯著輸給 Slint 且修不掉 → **改 Slint**

不存在「兩個都不行」的選項：兩者皆失敗會啟動緊急檢討，重新評估 egui 或 winit+wgpu 裸寫的可行性。

---

## Phase 3 — MVP

**Duration:** 4–5 週

### 模組

| Crate | 內容 |
|---|---|
| `mm-core` | `PageSource` async trait、`PageData`、`Thumbnail`、共用型別 |
| `mm-plugin-sdk` | Rust crate，定義 WASM plug-in trait + ABI（給插件作者用） |
| `mm-plugin-host` | wasmtime 載入器、權能管理、統一 `FormatPlugin` 介面 |
| `mm-source` | `LocalFolderSource`、`ZipSource`（內建 source impl） |
| `mm-decode` | rayon worker pool、LRU GPU texture cache、預讀 N±2 策略 |
| `mm-keymap` | Action enum、TOML 持久化、預設 binding（模仿原版） |
| `mm-config` | TOML 設定載入、portable 路徑解析、env override |
| `mm-recent` | 最近開過清單（JSON，< 100 entries，不需 SQLite） |
| `mm-ui` | Iced UI 層（隔離區，唯一 import iced 的 crate） |
| `mm-cli` | bin crate，組合一切，CLI 入口 |

### 功能

- 開啟單一檔案 / 資料夾（CLI 參數 + 拖放）
- 雙頁 / 單頁切換
- 右翻書（日漫模式）/ 左翻書（西式模式）切換
- 適應視窗 / 實際大小 / 適應寬度 / 適應高度
- 鍵盤翻頁（PgUp/PgDn/方向鍵）+ 跳首尾（Home/End）
- 預設 keymap 模仿原版（可從 TOML 改）
- 最近開過清單（自動填入、可手動清除）
- 即時縮圖（hover / 鍵盤焦點）— 不預掃

### 通過標準

- 能完整讀完一本 manga 不出錯
- 主觀速度感「跟原版マンガミーヤ一樣快」
- 在 Windows 上測一台普通 USB stick（HDD-level latency）依然順暢

---

## Phase 4 — Plugin Ecosystem + Polish

**Duration:** 3 週

### Plug-in

- 官方 WASM 插件：RAR、7z、PDF（pdfium WASM port 或 mupdf）
- Susie `.spi` shim（Windows only，相容遷移路徑）
- Plug-in discovery（掃描 `./plugins/`）
- Plug-in 設定 UI（啟用/停用、優先序、版本顯示）

### Polish

- Lanczos / Mitchell GPU fragment shader（取代 wgpu 預設 bilinear）
- 即時縮圖 memory LRU 快取（不落地）
- 設定 UI（keymap editor、濾鏡選擇、預讀策略、字型）
- 拖放開檔、剪貼簿貼圖、Windows context menu

### 通過標準

- 至少 3 個官方 WASM 插件可用（zip/cbz 內建不算）
- Susie shim 能載入常見 .spi（aimg、ax7z、unrar.spi 等）
- 縮放濾鏡可從設定切換並肉眼可見差異

---

## Phase 5 — CI/CD + Packaging

**Duration:** 1.5 週

### CI

- GitHub Actions matrix：`windows-latest`、`macos-latest`、`ubuntu-latest`
- Job：fmt → clippy → unit test → integration test → 截圖回歸 → build → artifact
- Windows: `-C target-feature=+crt-static`
- macOS: universal binary (`aarch64` + `x86_64`)
- Linux: 單一動態 binary + AppImage

### Portable layout

```
mangameeya/
├─ mangameeya.exe (或對應平台 binary)
├─ mangameeya.toml      # 預設設定
├─ keymap.toml          # 預設 keymap
├─ plugins/             # WASM 插件
├─ susie/               # (Windows) .spi 插件
└─ cache/               # 縮圖、解碼 cache (執行時生成)
```

### Release

- GitHub Releases 自動發佈（tag-driven）
- 三平台 zip：`mangameeya-{version}-{platform}.zip`
- 自動 update check（讀 GH Releases API，僅提示不下載）

### Docs

- mdBook source → GitHub Pages
- 內容：使用者文件、插件作者文件、keymap 自訂指南

### 通過標準

- 拿 zip 到完全乾淨的 OS（VM）執行：零安裝、雙擊即跑
- CI 全綠至少跑 1 週無 flake

---

## Phase 6 — OPDS Client（post-MVP）

**Duration:** 2–3 週

### 目的

把線上 manga server 生態接進來。一個 OPDS source = 一個 server URL。

### 範圍

- OPDS 1.2 + 2.0 catalog 解析
- HTTP source：Suwayomi、Komga、Kavita、calibre-web 共通
- 認證：Basic auth、token / API key
- 串流分頁（不全量下載章節）
- 縮圖代理快取
- `RemoteSource` 實作 `PageSource` async trait（架構零修改）

### 通過標準

- 接上一台 Suwayomi 能讀
- 接上一台 Komga 能讀
- 切換 source 不需重啟

---

## Phase 7+ — 暫不規劃

- 自家 WASM remote source plug-in
- 行動裝置
- 雲端同步進度
- 翻譯整合（OCR）

待 Phase 6 完成、有實際使用者回饋後再評估。
