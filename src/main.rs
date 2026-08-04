mod cli;
mod ingestor;
mod model;
mod storage;
mod ui;
mod updater;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use storage::Database;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let db = Database::open()?;

    // Auto scan local Copilot & AGY logs ONLY on first run when DB is empty for instant startup
    if db.is_empty()? {
        let _ = ingestor::scan_and_ingest(&db, None, false);
    }

    match &cli.command {
        Some(Commands::Recall { query, local, pinned, text }) => {
            let interactive = !(*text);
            handle_recall(&db, query.as_deref(), *local, *pinned, interactive)?;
        }
        Some(Commands::Pin { id, note: _ }) => {
            handle_pin(&db, *id)?;
        }
        Some(Commands::Scan { path, reindex, verbose }) => {
            println!("🔍 正在掃描本機 Copilot CLI 與 AGY CLI 對話日誌...");
            if *reindex {
                println!("🧹 清空舊索引資料庫，重新建立精準對話紀錄...");
                let _ = db.clear_all();
            }
            let count = ingestor::scan_and_ingest(&db, path.as_deref(), *verbose)?;
            println!("✅ 成功索引並更新 {} 筆全新歷史解法！", count);
        }
        Some(Commands::Stats) => {
            let stats = db.get_stats()?;
            ui::render_stats(&stats);
        }
        Some(Commands::Update { force }) => {
            updater::check_and_update(*force)?;
        }
        Some(Commands::Version) => {
            println!("cpl version {}", env!("CARGO_PKG_VERSION"));
        }
        None => {
            // Default behavior when typing just `cpl`: Open interactive TUI menu
            handle_recall(&db, None, false, false, true)?;
        }
    }

    Ok(())
}

fn handle_recall(db: &Database, query: Option<&str>, local_only: bool, pinned: bool, interactive: bool) -> Result<()> {
    let project_filter = if local_only {
        std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().to_string())
    } else {
        None
    };

    let initial_solutions = db.search_paged(query, project_filter.as_deref(), pinned, 0, 50)?;
    ui::render_results(&initial_solutions, db, interactive, project_filter.as_deref(), pinned, query)?;
    Ok(())
}

fn handle_pin(db: &Database, target_id: Option<i64>) -> Result<()> {
    let id_to_pin = match target_id {
        Some(id) => id,
        None => match db.get_latest_id()? {
            Some(latest) => latest,
            None => {
                println!("❌ 資料庫中尚無可供收藏的解法紀錄。");
                return Ok(());
            }
        },
    };

    let is_pinned = db.toggle_pin(id_to_pin)?;
    if is_pinned {
        println!("⭐ 成功星號收藏解法紀錄 #{}！", id_to_pin);
    } else {
        println!("📌 已取消收藏解法紀錄 #{}！", id_to_pin);
    }

    Ok(())
}
