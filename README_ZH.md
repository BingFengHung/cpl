# 🚀 cpl (Copilot Plus): Copilot CLI 本機極速快取與 1 秒執行加速器 (v1.0.0)

[English](README.md) | [繁體中文](README_ZH.md)

> **0.001 秒本機快取優先 ➔ 無縫轉接雲端 AI ➔ 一鍵直接執行的極速 CLI 加速器。**

`cpl` (Copilot Plus) 是一個使用 Rust 開發的高效能 CLI 工具。它作為 **GitHub Copilot CLI** 與 **Google Antigravity CLI (`agy`)** 前端的 **「本機極速快取層 (Local Fast Cache Layer)」**。

告別每次搜尋重複指令都要等待 5 秒鐘的雲端 API 網路延遲！`cpl` 在本機 0.001 秒發送 SQLite FTS5 檢索，找到解法後按下 `Enter` 即可直接在終端機中執行指令。若本機未命中，自動平滑轉發雲端 Copilot CLI。

---

## ✨ 核心優勢與亮點

- ⚡ **0.001 秒極速回應 (<1ms)**：80% 日常重複指令無需等待雲端 API 封包，本機直接 0 秒輸出。
- 🚀 **1 秒直接執行 (1-Click Execution)**：搜尋到指令後按下 `Enter` 直接在終端機中運行命令，無縫順暢！
- 🌐 **本機快取優先 ➔ 雲端 AI 備援**：本機未命中時自動轉接 `gh copilot suggest` 詢問並自動學習存庫。
- 🔄 **0 學習成本單一入口**：直接打 `cpl "需求"` 即可，不需記憶複雜子命令。
- 🔌 **100% 離線可用**：高鐵、斷網或限制性防火牆環境下依然能搜尋並執行歷史經驗。
- 🔄 **一鍵自我更新 (`cpl --version`)**：透過 GitHub Releases API 一鍵無縫升級！

---

## 📦 快速安裝（無需本機安裝 Rust）

只需直接從 **[GitHub Releases 頁面](https://github.com/BingFengHung/cpl/releases)** 下載編譯好的檔案即可。

### Windows
從 Releases 下載 `cpl-windows-amd64.exe`，將其重命名為 `cpl.exe` 並加入系統 `PATH` 環境變數中。

### Linux / macOS
```bash
curl -L https://github.com/BingFengHung/cpl.git -o cpl
chmod +x cpl
sudo mv cpl /usr/local/bin/
```

---

## ⚡ 使用方式 (極簡零負擔)

```bash
# 1. 0.001 秒本機搜尋與一鍵執行
cpl "壓縮圖片"
cpl "docker host連線"

# 2. 開啟全螢幕雙欄 TUI 搜尋器 (支援即時打字與無限捲動)
cpl

# 3. 強制重新掃描與建立索引
cpl --reindex
```

### TUI 介面快捷鍵
- `直接打字`：即時關鍵字動態搜尋 (Live Fuzzy Search)
- `↑ / ↓`：上下切換選取項目
- `Enter`：**1 秒直接在終端機中執行該指令**
- `c`：複製選中指令至剪貼簿
- `Esc`：退出介面
