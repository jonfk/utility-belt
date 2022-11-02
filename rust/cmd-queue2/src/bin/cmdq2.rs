use clap::{Parser, Subcommand};
use cmd_queue2::error::CmdqError;
use std::path::PathBuf;

fn main() -> Result<(), CmdqError> {
    tracing_subscriber::fmt::init();

    let cli_args = CliArgs::parse();

    match cli_args.commands {
        CliSubCommands::Ytdlp { filepath } => cmd_queue2::run_ytdlp_file(PathBuf::from(filepath))?,
    }
    Ok(())
}

#[derive(Debug, Parser)]
#[command(name = "cmdq")]
#[command(about = "A program to queue commands", long_about = None)]
struct CliArgs {
    #[command(subcommand)]
    commands: CliSubCommands,
}

#[derive(Debug, Subcommand)]
enum CliSubCommands {
    Ytdlp { filepath: String },
}
