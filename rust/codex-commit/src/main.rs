mod assets;
mod cli;
mod codex;
mod error;
mod flow;
mod git;
mod message;
mod prompt;
mod proposal;

use clap::Parser;

use crate::cli::Cli;

fn main() {
    let cli = Cli::parse();

    if let Err(report) = flow::run(cli) {
        eprintln!("{report:?}");
        std::process::exit(1);
    }
}
