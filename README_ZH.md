# 🚀 cpl (Copilot Plus): AI 歷史解法記憶與極速檢索 CLI

[English](README.md) | [繁體中文](README_ZH.md)

> **零冷啟動 (Zero Cold-Start) 的本機 AI 歷史對話記憶庫與一鍵自我更新助手。**

`cpl` 是一個使用 Rust 開發的高效能 CLI 工具。它能自動將您電腦中過往的 **GitHub Copilot CLI** 以及 **Google Antigravity CLI (`agy`)** 對話紀錄建立為本機 SQLite FTS5 全文檢索索引。讓您無需重新輸入 Prompt 或複製貼上，就能在終端機中 0 秒快速搜尋、預覽與帶入過往成功的 AI 指令與程式碼片段。

---

## ✨ 核心功能

- **⚡ 零冷啟動 (Zero Cold-Start)**：安裝後首刷自動回溯讀取您電腦上現有的 Copilot CLI 歷史日誌並建立索引。
- **🔍 fzf 風格雙欄 TUI 搜尋**：提供高刷終端機雙欄介面，支援即時關鍵字過濾與程式碼語法高亮預覽。
- **📋 一鍵複製至剪貼簿**：在選單中按下 `Enter` 鍵，自動將選中的指令複製至系統剪貼簿。
- **📌 關鍵解法一鍵收藏**：將優秀的 AI 指令或程式碼標記為星號收藏 (`cpl pin`)，方便日後隨時呼叫。
- **🔄 自動自我更新 (`cpl update`)**：無需重新手動下載檔案或安裝 Rust 環境，透過 GitHub Releases API 一鍵升級至最新版本！
- **🌐 雲端跨平台編譯**：透過 GitHub Actions 自動編譯 Windows (`.exe`)、Linux 與 macOS (Apple Silicon/Intel) 二進位執行檔。

---

## 📦 快速安裝（無需本機安裝 Rust）

您**不需要**在電腦上安裝 Rust 環境！只需直接從 **[GitHub Releases 頁面](https://github.com/BingFengHung/cpl/releases)** 下載編譯好的檔案即可。

### Windows
從 Releases 下載 `cpl-windows-amd64.exe`，將其重命名為 `cpl.exe` 並加入系統 `PATH` 環境變數中。

### Linux / macOS
```bash
# 下載二進位檔、賦予執行權限並移動至 bin 目錄
curl -L https://github.com/BingFengHung/cpl/releases/latest/download/cpl-linux-amd64 -o cpl
chmod +x cpl
sudo mv cpl /usr/local/bin/
```

---

## ⚡ 命令說明與快捷鍵

| 指令 | 功能說明 |
| :--- | :--- |
| `cpl` / `cpl recall` | 打開 TUI 互動式歷史對話與指令搜尋介面 |
| `cpl recall <關鍵字>` | 搜尋包含特定關鍵字的歷史解法 |
| `cpl recall -g` | 跨所有專案進行全域歷史搜尋 |
| `cpl recall -p` | 僅顯示已星號收藏 (`pinned`) 的解法 |
| `cpl pin` | 收藏最近一次的 Copilot CLI 解法 |
| `cpl scan` | 手動觸發對話日誌掃描與索引更新 |
| `cpl update` | **自動從 GitHub Releases 升級 `cpl` 至最新版本** |
| `cpl version` | 顯示當前 `cpl` 版本號 |

### TUI 介面快捷鍵
- `↑ / ↓` 或 `k / j`：上下切換搜尋結果
- `Enter`：複製選中的指令至剪貼簿
- `q / Esc`：退出介面

---

## 🔄 自我更新機制

使用以下命令保持 `cpl` 為最新版本：
```bash
cpl update
```
`cpl` 會自動向 GitHub Releases API 查詢最新發布版本，下載適合您作業系統的二進位檔並完成自動原地替換！

---

## 🛠️ 開發與 CI/CD 發布流程

推送版本標籤時：
```bash
git tag v0.1.0
git push origin v0.1.0
```
GitHub Actions ([`.github/workflows/release.yml`](.github/workflows/release.yml)) 會自動啟動 Linux、Windows 與 macOS 的雲端編譯，並將產出的二進位檔案直接發布至 GitHub Release 頁面！
