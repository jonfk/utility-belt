use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "ghostty-session-manager",
    about = "Treat Ghostty windows like project sessions",
    version
)]
pub struct Cli {
    #[arg(long, global = true, help = "Print verbose diagnostics to stderr")]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List Ghostty windows and derived project paths
    Ls {
        #[arg(long, help = "Render the live inventory as JSON")]
        json: bool,
    },
    /// Open an interactive picker and focus a Ghostty window
    Switch,
}
