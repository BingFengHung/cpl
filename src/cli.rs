use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "cpl",
    author = "Copilot CLI Plus Developer",
    version = "1.0.0",
    about = "cpl: Copilot CLI Local Fast-Cache & Smart Execution Accelerator"
)]
pub struct Cli {
    /// Natural language demand or search keyword (e.g. cpl "compress png" or cpl docker)
    pub query: Option<String>,

    /// Display plain text list instead of interactive TUI menu
    #[arg(short, long)]
    pub text: bool,

    /// Force re-indexing local Copilot & AGY CLI transcript logs from scratch
    #[arg(short, long)]
    pub reindex: bool,

    /// Show version details
    #[arg(short, long)]
    pub version: bool,
}
