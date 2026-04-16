mod application;
mod cli;
mod debug_profile;
mod domain;
mod error;
mod ghostty;
mod state;
mod tui;

use clap::Parser;

use crate::cli::Cli;
use crate::debug_profile::DebugProfiler;

fn main() {
    let cli = Cli::parse();
    let verbose = cli.verbose;
    let debug = cli.debug;
    let profiler = DebugProfiler::new(debug);

    let result = profiler.run(|| cli::run(cli));
    if let Err(error) = profiler.print_report() {
        eprintln!("Failed to write debug timing report: {error}");
    }

    if let Err(report) = result {
        if verbose {
            eprintln!("{report:?}");
        } else {
            eprintln!("{report}");
        }
        std::process::exit(1);
    }
}
