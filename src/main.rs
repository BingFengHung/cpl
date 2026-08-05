mod cli;
mod executor;
mod ingestor;
mod model;
mod storage;
mod ui;
mod updater;

use anyhow::Result;
use clap::Parser;
use cli::Cli;
use storage::Database;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let db = Database::open()?;

    // Auto scan local Copilot & AGY logs ONLY on first run when DB is empty for instant startup
    if db.is_empty()? || cli.reindex {
        if cli.reindex {
            println!("🧹 清空舊索引資料庫，重新建立精準對話紀錄...");
            let _ = db.clear_all();
        }
        let _ = ingestor::scan_and_ingest(&db, None, false);
    }

    if cli.version {
        println!("cpl version {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    match cli.query {
        Some(query) if !query.trim().is_empty() => {
            // Instant 0.001s Local Sub-millisecond Lookup
            let solutions = db.search_paged(Some(&query), None, false, 0, 5)?;
            if let Some(best_match) = solutions.first() {
                executor::prompt_and_execute_solution(best_match)?;
            } else {
                // Fallback to cloud AI
                executor::fallback_cloud_ai(&query)?;
            }
        }
        _ => {
            // Default behavior: Open fast interactive TUI fuzzy searcher
            let initial_solutions = db.search_paged(None, None, false, 0, 50)?;
            ui::render_results(&initial_solutions, &db, !cli.text, None, false, None)?;
        }
    }

    Ok(())
}
