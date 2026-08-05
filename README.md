# 🚀 cpl (Copilot Plus): Copilot CLI Local Fast-Cache & Direct Execution Engine (v1.0.0)

[English](README.md) | [Traditional Chinese](README_ZH.md)

> **Sub-millisecond local cache first ➔ Seamless cloud AI fallback ➔ 1-click direct shell execution.**

`cpl` (Copilot Plus) is a high-performance CLI written in Rust. It serves as an ultra-fast **Local Fast-Cache Layer** for **GitHub Copilot CLI** and **Google Antigravity CLI (`agy`)**.

Say goodbye to waiting 3–8 seconds for cloud network latency every time you ask for a familiar command! `cpl` queries your local SQLite FTS5 index in **<1ms (0.001s)**. Pressing `Enter` executes the command directly in your shell. If a query misses locally, it seamlessly falls back to cloud Copilot CLI and indexes the result for next time.

---

## ✨ Features & Highlights

- ⚡ **0.001s Sub-millisecond Latency (<1ms)**: 80% of daily recurring CLI demands return instantly without waiting for cloud network roundtrips.
- 🚀 **1-Click Direct Shell Execution**: Press `Enter` to run commands directly in your terminal.
- 🌐 **Local Cache First ➔ Cloud AI Fallback**: Seamlessly forwards unmatched queries to `gh copilot suggest` and auto-indexes the solution locally.
- 🔄 **Zero Learning Curve**: Unified `cpl "demand"` interface without needing complex subcommands.
- 🔌 **100% Offline Compatible**: Search and execute past solutions even when offline or behind firewalls.
- 🔄 **Self-Updating**: Upgrade instantly to the latest binary using GitHub Releases API.

---

## ⚡ Usage

```bash
# 1. Sub-millisecond local cache hit & 1-click execution
cpl "compress png"
cpl "docker host connection"

# 2. Open interactive TUI searcher with live fuzzy search & infinite scroll
cpl

# 3. Force re-indexing local transcript logs
cpl --reindex
```

### TUI Keybindings
- `Type letters`: Live interactive fuzzy search
- `↑ / ↓`: Navigate selection
- `Enter`: **1-click direct execution in shell**
- `c`: Copy command to clipboard
- `Esc`: Exit
