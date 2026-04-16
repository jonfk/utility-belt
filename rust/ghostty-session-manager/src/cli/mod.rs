mod args;
mod json;
mod table;

use std::io::{self, Write};

use error_stack::{Report, ResultExt};

use crate::application::{SwitchWindow, complete_switch, list_windows, prepare_switch};
use crate::error::AppError;
use crate::ghostty::GhosttyClient;
use crate::state::StateStore;
use crate::tui::{self, PickerEntry, PickerOutcome};

pub use args::Cli;
use args::Command;

pub fn run(cli: Cli) -> Result<(), Report<AppError>> {
    let ghostty = GhosttyClient::new(cli.verbose);
    let state_store = StateStore::from_default_path()?;

    match cli.command {
        Command::Ls { json: render_json } => run_ls(&ghostty, &state_store, render_json),
        Command::Switch => run_switch(&ghostty, &state_store),
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

fn run_switch(ghostty: &GhosttyClient, state_store: &StateStore) -> Result<(), Report<AppError>> {
    let mut context = prepare_switch(ghostty, state_store)?;
    let entries = context
        .windows
        .iter()
        .map(picker_entry_from_window)
        .collect();

    match tui::run_picker(entries)? {
        PickerOutcome::Confirm(entry) => {
            let selection = context
                .windows
                .iter()
                .find(|window| window.window_id == entry.window_id)
                .ok_or_else(|| {
                    Report::new(AppError::Tui)
                        .attach("Selected picker row no longer matches a Ghostty window")
                })?;
            complete_switch(ghostty, state_store, &mut context.state, selection)
        }
        PickerOutcome::Cancel => Ok(()),
    }
}

fn write_stdout(rendered: &str) -> Result<(), Report<AppError>> {
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{rendered}")
        .change_context(AppError::Output)
        .attach("Failed to write command output to stdout")?;
    Ok(())
}

fn picker_entry_from_window(window: &SwitchWindow) -> PickerEntry {
    PickerEntry {
        window_id: window.window_id.clone(),
        title: window.title.clone(),
        detail: window.detail.clone(),
    }
}
