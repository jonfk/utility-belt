mod app;
mod cli;
mod domain;
mod error;
mod ghostty;

use clap::Parser;

use crate::cli::Cli;

fn main() {
    let cli = Cli::parse();
    let verbose = cli.verbose;

    if let Err(report) = app::run(cli) {
        if verbose {
            eprintln!("{report:?}");
        } else {
            eprintln!("{report}");
        }
        std::process::exit(1);
    }
}
