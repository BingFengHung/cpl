# 🚀 cpl (Copilot Plus): AI Solution Recall & Instant Indexing CLI

> **Zero Cold-Start local AI memory database & self-updating assistant for Copilot CLI users.**

`cpl` is a blazingly fast CLI tool written in Rust that automatically indexes past GitHub Copilot CLI transcript logs into a local SQLite FTS5 database, allowing you to instantly recall, search, and execute past AI-generated commands and code snippets without re-prompting or copy-pasting.

---

## ✨ Features

- **⚡ Zero Cold-Start**: Immediately indexes existing Copilot CLI logs on your machine upon first run.
- **🔍 fzf-style TUI Search**: Interactive dual-pane terminal UI with syntax highlighting and instant fuzzy search.
- **📋 1-Click Clipboard Copy**: Select any recalled command and press `Enter` to copy directly to your clipboard.
- **📌 Pin / Bookmark**: Star your favorite AI solutions and commands (`cpl pin`).
- **🔄 Auto Self-Update (`cpl update`)**: Upgrade to the latest release directly from GitHub Releases without installing Rust or downloading files manually!
- **🌐 Cross-Platform CI**: Automated cross-compilation via GitHub Actions for Windows, Linux, and macOS.

---

## 📦 Quick Installation (No Rust Required)

You do **not** need Rust installed locally to run `cpl`! Simply download the precompiled binary for your OS directly from the **[GitHub Releases Page](https://github.com/BingFengHung/cpl/releases)**.

### Windows
Download `cpl-windows-amd64.exe` from Releases, rename to `cpl.exe`, and add to your System `PATH`.

### Linux / macOS
```bash
# Download binary, make executable, and move to bin path
curl -L https://github.com/BingFengHung/cpl/releases/latest/download/cpl-linux-amd64 -o cpl
chmod +x cpl
sudo mv cpl /usr/local/bin/
```

---

## ⚡ Usage & Keybindings

| Command | Action |
| :--- | :--- |
| `cpl` / `cpl recall` | Open interactive TUI solution recall search |
| `cpl recall <keyword>` | Search solutions matching keyword |
| `cpl recall -g` | Search solutions globally across all projects |
| `cpl recall -p` | Show pinned / starred solutions only |
| `cpl pin` | Star/pin the latest AI solution |
| `cpl scan` | Manually trigger log re-indexing |
| `cpl update` | **Self-update `cpl` to latest GitHub Release version** |
| `cpl version` | Display current `cpl` version |

### TUI Keybindings
- `↑ / ↓` or `k / j`: Navigate solution list
- `Enter`: Copy selected command to clipboard
- `q / Esc`: Quit

---

## 🔄 Self-Update

Keep `cpl` up-to-date with one command:
```bash
cpl update
```
`cpl` queries the GitHub Releases API, downloads the correct compiled binary for your OS, and replaces itself automatically!

---

## 🛠️ GitHub Actions CI/CD Setup

To publish a new release:
```bash
git tag v0.1.0
git push origin v0.1.0
```
GitHub Actions (`.github/workflows/release.yml`) will automatically build Linux, Windows, and macOS binaries and publish them directly to the GitHub Release page!
