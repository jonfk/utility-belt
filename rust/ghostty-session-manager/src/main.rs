mod application;
mod cli;
mod domain;
mod error;
mod ghostty;
mod state;

use clap::Parser;

use crate::cli::Cli;

fn main() {
    let cli = Cli::parse();
    let verbose = cli.verbose;

    if let Err(report) = cli::run(cli) {
        if verbose {
            eprintln!("{report:?}");
        } else {
            eprintln!("{report}");
        }
        std::process::exit(1);
    }
}
