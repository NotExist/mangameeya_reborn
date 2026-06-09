# Backlog / 待改進事項

Phase 階段中發現、但決定延後處理的優化項。每筆紀錄：**問題**、**延後原因**、**改進方向與方法**、**重新評估的觸發條件**。

---

## JPEG 解碼效能 (Phase 1a 發現 → Phase 4 重新評估)

**問題**

Phase 1a baseline 量測（GHA ubuntu-latest）：

| 場景 | 實測 | 目標 | 超出 |
|---|---|---|---|
| 2400×3400 JPEG cold decode (p50) | 75.6ms | 60ms | +26% |
| 6000×4000 JPEG cold decode (criterion mean) | 222.5ms | 200ms | +11% |

兩次跑同樣程式碼 cold decode 變動 ~12%（67→75ms、197→222ms），代表測量環境本身雜訊大。

**延後原因**

1. **真實使用情境下成本被吸收**：reader 主要走 N±2 預讀 + GPU texture cache，cold decode 只發生在開檔首頁與跳頁。Startup→首頁 79ms（目標 500ms），有 6× 餘裕，可吸收個別頁的 cold decode 偏慢。
2. **GHA runner 是 4-core 共享 VM**，與真實桌面 CPU 表現可能落差顯著，現在判定 decode 不過關可能是測量平台問題而非演算法問題。
3. Resize 已經攻克（fast_image_resize 帶來 7-9× 提速）。Pipeline 主要瓶頸已移除。

**改進方向與方法**

按優先序：

1. **重新量測於真實桌面**（Phase 1b 或 Phase 3 MVP 期間）。如果在 desktop-class CPU 上 cold decode 6000×4000 < 100ms，**問題不存在**，本項關閉。
2. **追新版 zune-jpeg**：`image` 0.25 鎖定特定 zune-jpeg 版本。zune-jpeg 持續 SIMD 優化中，可能新版已快。方法：`image` 升 patch 或透過 `[patch.crates-io]` 強制使用 zune-jpeg HEAD。**零外部依賴。**
3. **引入 `turbojpeg-sys` 靜態鏈接 libjpeg-turbo**：典型快 2-3×。GHA 各平台 runner 預裝 CMake；libjpeg-turbo 原始碼 vendored、靜態鏈接到 binary，runtime 仍零依賴。代價：build matrix 複雜度增加、CI 跑時間從 ~2 分鐘拉到 5-8 分鐘、release size +200KB。
4. **GPU 端 JPEG 解碼**（NVDEC / VideoToolbox / Media Foundation）：硬體加速，極快但跨平台複雜。僅當前述都不夠時考慮。

**重新評估觸發條件**

任一發生即重啟此項：
- Phase 1b 或 Phase 3 在真實 desktop CPU 量到 cold decode 仍超目標
- 使用者主觀回饋「翻頁有 hitching、感覺解碼慢」
- 開發過程發現 decode 同樣是 Phase 4+ 其他熱路徑的瓶頸

**目前狀態**：觀察期，不主動優化。

---

## Legacy Windows (Win7) build 可行性驗證 (Phase 5 deliverable)

**問題**

開發者本人是 Win7 使用者，原版 MangaMeeya 不再維護、Unicode 等問題無法修正。需要本專案能在 Win7 跑出可用版本。XP **不支援**（DX9-only 系統、Rust 已徹底拔除 XP target）。

**架構面已預留**

- `PageRenderer` trait 抽象（[architecture.md](architecture.md#render-backend-abstraction-pagerenderer-trait)）：default `WgpuRenderer`，legacy 換 `SoftwareRenderer` (softbuffer + tiny-skia)
- Cargo feature `legacy-windows` 啟用時走軟體渲染
- 核心 crate（core / source / decode / filter / keymap / config）平台無關
- Unicode 透過 Rust `OsString` / UTF-8 默認解決，無需特殊處理

**改進方向與方法**

按優先序：

1. **Phase 5 — Cross-compile sanity check (GHA)**
   - `rustup target add x86_64-win7-windows-msvc`（Tier 3，無自動測試）
   - 在 GHA 嘗試 `cargo build --release --no-default-features --features legacy-windows --target x86_64-win7-windows-msvc`
   - 列出每個依賴，標記哪些 Win10+ API 需要 conditional 規避
   - 失敗依賴清單回頭 patch / fork / 替換

2. **Phase 5 — Win7 user 實機驗證**
   - 拿 cross-compile 出來的 binary 到開發者本人的 Win7 機器
   - 跑 [test-checklist.md](test-checklist.md) 的 Win7 子集
   - 主要驗證項：能啟動、能開檔、CJK 檔名顯示正確、鍵盤翻頁可用、效能可接受（< 100ms 翻頁）
   - 預期失敗點記錄在這份 backlog

3. **若有依賴 crate 阻擋**
   - winit 0.30：可能呼叫 `SetProcessDpiAwarenessContext` (Win10+)。檢查是否有 fallback；若無，patch 或 fork 加 conditional
   - iced 0.14：依賴 wgpu，但 tiny-skia backend 是純 CPU。確認 tiny-skia 後端啟用方式
   - softbuffer：應該支援 Win7（用古老的 BeginPaint/StretchDIBits API）
   - 個別 crate 走 `[patch.crates-io]` 暫時 fork

4. **Release 流程整合**
   - GHA artifact `hamana-{version}-win7-best-effort.zip`
   - Release notes 明確標 best-effort、unsupported、社群維護優先
   - 提供回報 issue template 給其他 Win7 user

**重新評估觸發條件**

任一發生即重新檢視策略：
- Phase 5 cross-compile 在第一輪驗證後 binary 仍無法在 Win7 啟動，且 patch 成本過高（>2 週工作量）→ 考慮放棄 Win7 支援
- 依賴 crate 重大改版引入 Win10+ hard dependency 無 workaround → 重評估
- 使用 Win7 的真實人數驗證為零（即便發 release 也沒人下載 legacy artifact）→ 重評估

**目前狀態**：架構已就位，等 Phase 5 啟動實作。

---

## (Template — 未來新增條目)

**問題**：

**延後原因**：

**改進方向與方法**：

**重新評估觸發條件**：
