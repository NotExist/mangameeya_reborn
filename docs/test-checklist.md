# 實體機器測試清單

CI 上跑不出來的事項，集中在這裡，待你有實體 Windows / Mac / Linux 桌面時逐項驗證後回報。**這份文件本身就是 baseline 紀錄表**——測完數字直接填回對應位置。

每個項目格式：**目的 → 步驟 → 記錄欄 → 通過條件**

---

## 0. 共通準備

### 0.1 取得 binary

從 GHA 抓最新 CI artifact，或 local clone + `cargo build --release --features all-spikes`：

- spike_gpu — Phase 1b 純 winit+wgpu，**延遲天花板基準**
- spike_iced — Phase 2 Iced+image widget，**框架成本量測**

### 0.2 準備 fixture

bench fixture 不在 repo。三選一：
- **option A (推薦)**：拿一本實際 manga zip（含 CJK 檔名），記下絕對路徑
- **option B**：執行 `cargo run --release --bin gen_fixture -- ./bench-fixture.zip` 產生合成 fixture（~1.3GB，無 CJK 檔名）
- **option C**：設環境變數 `MANGAMEEYA_BENCH_FIXTURE=/path/to/your.zip`

兩個 spike binary 都接受 `<fixture-path>` 為第一參數或讀 env。

### 0.3 記錄環境

| 項目 | 值 |
|---|---|
| 平台 (Win/Mac/Linux + 版本) | _____ |
| CPU 型號 | _____ |
| GPU 型號 | _____ |
| 顯示器刷新率 (60/120/144Hz?) | _____ |
| 螢幕縮放百分比 (100/125/150%?) | _____ |
| 是否插電（筆電會降頻） | _____ |
| 日期 | _____ |

---

## 1. Phase 1b — spike_gpu (winit+wgpu 純原生)

### 1.1 啟動 + 首頁顯示

**目的**：確認程式能在你的平台啟動、開窗、顯示第一頁。
**步驟**：
```
./spike_gpu /path/to/fixture.zip
```
**記錄**：
- ✅ / ❌ 視窗開出來
- ✅ / ❌ 第一頁圖像正常顯示
- stderr 印出：`[spike_gpu] loaded N pages from ...`，N = _____
- 視覺感受：圖像清晰度（特別注意縮放後是否模糊或鋸齒）_____
- 啟動到第一頁可見的主觀感受秒數：_____

### 1.2 KeyDown → 畫面更新延遲（核心指標）

**目的**：量測**完整端到端**延遲（KeyDown OS event → 畫面變更）。stderr 印的是 app-side（KeyDown→queue.submit 返回），需另外加感官觀察。

**步驟**：
1. 啟動 spike_gpu
2. **左方向鍵連按 10 次**（這是 RTL 翻頁的「下一頁」）
3. 觀察每次按鍵到畫面變化的感覺
4. **長按左方向鍵 3 秒**測 key-repeat 連續翻頁
5. 退出（Esc），把 stderr 的 latency 行抓出來

**記錄**（從 stderr 抓 10 行 `[spike_gpu] KeyDown→submit-return: X.XXX ms`）：
```
1: _____ ms
2: _____ ms
...
中位數: _____ ms
最大: _____ ms
```

主觀觀察：
- 單擊翻頁是否「按下即翻」？_____ ✅/❌
- 長按連續翻頁是否流暢、無 stutter？_____ ✅/❌
- 視覺上有任何 frame skip / hitch？_____ ✅/❌

**通過條件**：
- 中位數 ≤ 16ms (60Hz 螢幕) 或 ≤ 8ms (120Hz)
- 最大值不超過中位數 2×
- 主觀無 stutter

### 1.3 大檔（6000×4000）翻到時的卡頓

**目的**：fixture 有 30 張 6000×4000 大圖（合成 fixture 是 page_221.jpg 之後）。翻到這些頁時是否明顯卡？

**步驟**：
1. 啟動 spike_gpu
2. 按 End 跳到最後頁
3. 按 Right (prev) 連按 10 次回頭到大圖區
4. 觀察每次翻頁延遲

**記錄**：
- 翻到大圖時 stderr 顯示的 latency：_____ ms
- 主觀感受：是否明顯比小圖頁慢？_____

**通過條件**：大圖頁 latency 不超過小圖頁 2×

### 1.4 視窗 resize 行為

**目的**：縮放視窗、最大化、全螢幕切換時不崩、畫面正常。

**步驟**：
1. 啟動
2. 拖視窗角落縮小到最小
3. 最大化
4. (可選) 全螢幕

**記錄**：
- ✅ / ❌ 不崩
- ✅ / ❌ 畫面正常重繪
- 任何視覺異常：_____

---

## 2. Phase 2 — spike_iced (Iced 框架)

### 2.1 啟動 + 首頁顯示

**步驟**：
```
./spike_iced /path/to/fixture.zip
```

**記錄**：
- ✅ / ❌ 視窗開出來
- ✅ / ❌ 第一頁圖像正常顯示
- ✅ / ❌ 下方狀態列顯示檔名
- 主觀：圖像清晰度與 spike_gpu 比較（Iced 用 image widget，內部 resize 演算法可能不同）_____

### 2.2 KeyDown → 畫面更新延遲 vs spike_gpu

**目的**：量測 Iced 框架在熱路徑上額外加了多少 overhead。

**步驟**：與 1.2 相同，但跑 spike_iced。

**記錄**：
```
spike_iced 中位數: _____ ms
spike_iced 最大: _____ ms
```

對比 1.2 的 spike_gpu 數字：
- Iced overhead (Iced 中位數 − GPU 中位數)：_____ ms
- 主觀感受是否能察覺 Iced 較慢？_____

**通過條件**：
- Iced overhead < 5ms（落在一個 vsync 內）
- 主觀仍「按下即翻」

### 2.3 CJK 檔名顯示（重要 — 影響框架決策）

**目的**：驗證 Iced 對 CJK 字型的 fallback。

**前置**：fixture 內至少有一個含日文 / 中文的檔名。如果合成 fixture，**請先手動準備**：例如把某個 manga zip 內的某頁改名為 `第１話「導入」.jpg`。

**步驟**：
1. 啟動 spike_iced 開到含 CJK 名的頁
2. 截圖狀態列

**記錄**：
- 截圖：[附圖位置]
- ✅ / ❌ 日文假名顯示正常
- ✅ / ❌ 中文（繁/簡）顯示正常
- ✅ / ❌ 全形括號、引號顯示正常
- ✅ / ❌ 沒有 tofu (□) 或缺字方塊
- 任何視覺缺陷：_____

**通過條件**：所有 CJK 字元正確渲染、無 tofu

### 2.4 IME 輸入測試（無法在此版實測，但記錄系統行為）

**狀況**：spike_iced 沒有輸入框可以打字。但 IME 啟動時的副作用（吃鍵、奪焦）可以模擬：

**步驟**：
1. 啟動 spike_iced
2. 開 IME（Win: Win+空白；macOS: Ctrl+Space；Linux: ibus/fcitx 視設定）
3. 切換到日文輸入法
4. 對著視窗按 Space、ArrowLeft、ArrowRight
5. 觀察是否仍能翻頁

**記錄**：
- IME 開啟時翻頁鍵是否仍生效？_____ ✅/❌
- IME 是否捕獲了非組字鍵？_____

**通過條件**：純導航鍵不被 IME 干擾

### 2.5 視窗 resize 行為

與 1.4 相同。

---

## 3. 比較與決策

### 3.1 spike_gpu vs spike_iced

填寫對比表：

| 指標 | spike_gpu | spike_iced | 差距 |
|---|---|---|---|
| 啟動到首頁可見 (主觀秒數) | _____ | _____ | _____ |
| KeyDown→畫面 中位數 | _____ ms | _____ ms | _____ ms |
| KeyDown→畫面 最大 | _____ ms | _____ ms | _____ ms |
| 主觀「按下即翻」感受 | _____ | _____ | _____ |
| 大圖頁 latency | _____ ms | _____ ms | _____ ms |
| CJK 檔名渲染 | N/A | _____ | _____ |
| 啟動時間 | _____ s | _____ s | _____ s |

### 3.2 結論建議

**對 Phase 2 框架定案的決策**：

選項 A：Iced overhead 可接受、CJK 全部通過 → **定案 Iced**
選項 B：Iced overhead 顯著（>5ms）或 CJK 失格 → 啟動 Slint 影子分支重來
選項 C：兩個都不滿意 → 緊急會議

你的結論：_____

---

## 4. 附帶觀察（自由記錄）

凡 binary 跑起來的任何不舒服、bug、想法都寫這裡，後續 Phase 規劃會吸收：

- _____
- _____
- _____

---

## 提交回 repo

測完後請把這份檔案連同數字 commit 回，或開 GitHub Issue 貼結果。
