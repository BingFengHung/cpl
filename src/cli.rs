use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "cpl",
    author = "Copilot CLI Plus Developer",
    version = "0.4.0",
    about = "Copilot CLI Plus (cpl): AI Solution Recall, Indexing & Self-Updating CLI Assistant"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Recall and search past AI solutions & commands interactively
    Recall {
        /// Search keyword or query
        query: Option<String>,

        /// Search current project only (defaults to global search across all projects)
        #[arg(short, long)]
        local: bool,

        /// Show pinned solutions only
        #[arg(short, long)]
        pinned: bool,

        /// Display plain text output instead of TUI menu
        #[arg(short, long)]
        text: bool,
    },

    /// Pin/Star a solution for quick access
    Pin {
        /// ID of the solution to pin (defaults to the most recent solution)
        id: Option<i64>,

        /// Optional note or tag for the pinned solution
        #[arg(short, long)]
        note: Option<String>,
    },

    /// Scan local Copilot CLI & AGY CLI transcripts and update search index
    Scan {
        /// Optional custom directory or file path to scan
        path: Option<String>,

        /// Force re-indexing all transcript logs from scratch
        #[arg(short, long)]
        reindex: bool,

        /// Print verbose debug details of all checked paths and files
        #[arg(short, long)]
        verbose: bool,
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
