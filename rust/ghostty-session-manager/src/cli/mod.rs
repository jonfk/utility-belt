mod args;
mod json;
mod table;

use std::io::{self, Write};

use error_stack::{Report, ResultExt};

use crate::application::list_windows;
use crate::error::AppError;
use crate::ghostty::GhosttyClient;
use crate::state::StateStore;

pub use args::Cli;
use args::Command;

pub fn run(cli: Cli) -> Result<(), Report<AppError>> {
    let ghostty = GhosttyClient::new(cli.verbose);
    let state_store = StateStore::from_default_path()?;

    match cli.command {
        Command::Ls { json: render_json } => run_ls(&ghostty, &state_store, render_json),
    }
}

fn run_ls(
    ghostty: &GhosttyClient,
    state_store: &StateStore,
    render_json: bool,
) -> Result<(), Report<AppError>> {
    let inventory = list_windows(ghostty, state_store)?;
    let rendered = if render_json {
        json::render_inventory(&inventory)?
    } else {
        table::render_inventory(&inventory)
    };

    write_stdout(&rendered)
}

fn write_stdout(rendered: &str) -> Result<(), Report<AppError>> {
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{rendered}")
        .change_context(AppError::Output)
        .attach("Failed to write command output to stdout")?;
    Ok(())
}
