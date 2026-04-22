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

    #[arg(
        long,
        global = true,
        help = "Print span-based timing diagnostics to stderr"
    )]
    pub debug: bool,

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

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, Command};

    #[test]
    fn parses_debug_flag_for_ls() {
        let cli = Cli::try_parse_from(["ghostty-session-manager", "--debug", "ls"])
            .expect("cli should parse");

        assert!(cli.debug);
        assert!(matches!(cli.command, Command::Ls { json: false }));
    }

    #[test]
    fn parses_debug_flag_for_switch() {
        let cli = Cli::try_parse_from(["ghostty-session-manager", "switch", "--debug"])
            .expect("cli should parse");

        assert!(cli.debug);
        assert!(matches!(cli.command, Command::Switch));
    }
}
