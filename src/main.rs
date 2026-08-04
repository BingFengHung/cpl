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

    // Auto scan local Copilot logs on startup to ensure Zero Cold Start
    let _ = ingestor::scan_and_ingest(&db);

    match &cli.command {
        Some(Commands::Recall { query, global, pinned }) => {
            handle_recall(&db, query.as_deref(), *global, *pinned)?;
        }
        Some(Commands::Pin { id, note: _ }) => {
            handle_pin(&db, *id)?;
        }
        Some(Commands::Scan { reindex: _ }) => {
            println!("🔍 Scanning Copilot CLI transcript logs...");
            let count = ingestor::scan_and_ingest(&db)?;
            println!("✅ Successfully indexed {} new solution(s) into database!", count);
        }
        Some(Commands::Update { force }) => {
            updater::check_and_update(*force)?;
        }
        Some(Commands::Version) => {
            println!("cpl version {}", env!("CARGO_PKG_VERSION"));
        }
        None => {
            // Default behavior if query provided as direct positional arg: cpl <query>
            if let Some(ref q) = cli.query {
                handle_recall(&db, Some(q.as_str()), cli.global, cli.pinned)?;
            } else {
                // Interactive recall mode
                handle_recall(&db, None, cli.global, cli.pinned)?;
            }
        }
    }

    Ok(())
}

fn handle_recall(db: &Database, query: Option<&str>, global: bool, pinned: bool) -> Result<()> {
    let current_dir = if global {
        None
    } else {
        std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().to_string())
    };

    let solutions = db.search(query, current_dir.as_deref(), pinned)?;
    ui::render_interactive(&solutions, db)?;
    Ok(())
}

fn handle_pin(db: &Database, target_id: Option<i64>) -> Result<()> {
    let id_to_pin = match target_id {
        Some(id) => id,
        None => match db.get_latest_id()? {
            Some(latest) => latest,
            None => {
                println!("❌ No solutions found in database to pin.");
                return Ok(());
            }
        },
    };

    let is_pinned = db.toggle_pin(id_to_pin)?;
    if is_pinned {
        println!("⭐ Pinned solution #{} successfully!", id_to_pin);
    } else {
        println!("📌 Unpinned solution #{}!", id_to_pin);
    }

    Ok(())
}
