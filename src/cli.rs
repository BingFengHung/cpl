use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "cpl",
    author = "Copilot CLI Plus Developer",
    version = "0.1.0",
    about = "Copilot CLI Plus (cpl): AI Solution Recall, Indexing & Self-Updating CLI Assistant"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Quick search shortcut: cpl <query>
    #[arg(index = 1)]
    pub query: Option<String>,

    /// Search across all projects globally instead of current project only
    #[arg(short, long)]
    pub global: bool,

    /// Show pinned solutions only
    #[arg(short, long)]
    pub pinned: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Recall and search past AI solutions & commands interactively
    Recall {
        /// Search keyword or query
        query: Option<String>,

        /// Search across all projects globally
        #[arg(short, long)]
        global: bool,

        /// Show pinned solutions only
        #[arg(short, long)]
        pinned: bool,
    },

    /// Pin/Star a solution for quick access
    Pin {
        /// ID of the solution to pin (defaults to the most recent solution)
        id: Option<i64>,

        /// Optional note or tag for the pinned solution
        #[arg(short, long)]
        note: Option<String>,
    },

    /// Scan local Copilot CLI transcripts and update search index
    Scan {
        /// Force re-indexing all transcript logs from scratch
        #[arg(short, long)]
        reindex: bool,
    },

    /// Check for updates and self-update to the latest release from GitHub
    Update {
        /// Force update even if already on the latest version
        #[arg(short, long)]
        force: bool,
    },

    /// Show current version details
    Version,
}
