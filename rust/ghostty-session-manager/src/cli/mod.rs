mod args;
mod json;
mod table;

use std::io::{self, Write};
use std::sync::mpsc;
use std::thread;

use error_stack::{Report, ResultExt};
use tracing::info_span;

use crate::application::{
    SwitchWindow, complete_switch, list_windows, prepare_switch, reconcile_switch_inventory,
};
use crate::domain::WindowInventory;
use crate::error::AppError;
use crate::ghostty::GhosttyClient;
use crate::state::StateStore;
use crate::tui::{self, PickerEntry, PickerOutcome, PickerRefresh, RefreshMessage};

pub use args::Cli;
use args::Command;

pub fn run(cli: Cli) -> Result<(), Report<AppError>> {
    let command_name = match &cli.command {
        Command::Ls { .. } => "ls",
        Command::Switch => "switch",
    };
    let command_span = info_span!("command", command = command_name);
    let ghostty = GhosttyClient::new(cli.verbose);
    let state_store = StateStore::from_default_path()?;

    match cli.command {
        Command::Ls { json: render_json } => {
            let _command_enter = command_span.enter();
            let run_span = info_span!("cli.run", command = "ls");
            let _run_enter = run_span.enter();
            run_ls(&ghostty, &state_store, render_json)
        }
        Command::Switch => run_switch(&ghostty, &state_store, &command_span),
    }
}

fn run_ls(
    ghostty: &GhosttyClient,
    state_store: &StateStore,
    render_json: bool,
) -> Result<(), Report<AppError>> {
    let span = info_span!(
        "cli.run_ls",
        renderer = if render_json { "json" } else { "table" }
    );
    let _enter = span.enter();
    let inventory = list_windows(ghostty, state_store)?;
    let rendered = if render_json {
        json::render_inventory(&inventory)?
    } else {
        table::render_inventory(&inventory)
    };

    write_stdout(&rendered)
}

fn run_switch(
    ghostty: &GhosttyClient,
    state_store: &StateStore,
    command_span: &tracing::Span,
) -> Result<(), Report<AppError>> {
    let run_span = info_span!("cli.run", command = "switch");
    let mut context = {
        let _command_enter = command_span.enter();
        let _run_enter = run_span.enter();
        prepare_switch(ghostty, state_store)?
    };
    let initial_projects = context.state.projects.clone();
    let entries = {
        let _command_enter = command_span.enter();
        let _run_enter = run_span.enter();
        let entries_span = info_span!("cli.build_picker_entries");
        let _entries_enter = entries_span.enter();
        context
            .windows
            .iter()
            .map(picker_entry_from_window)
            .collect()
    };
    let mut latest_live_inventory: Option<WindowInventory> = None;
    let mut refresh_dirty = false;
    let refresh_receiver = if context.seeded_from_live {
        None
    } else {
        let (sender, receiver) = mpsc::channel();
        let refresh_ghostty = ghostty.clone();
        thread::spawn(move || {
            let message = match refresh_ghostty.query_windows() {
                Ok(inventory) => RefreshMessage::Success(inventory),
                Err(error) => RefreshMessage::Failure(format!("{error:?}")),
            };
            let _ = sender.send(message);
        });
        Some(receiver)
    };

    match tui::run_picker(
        entries,
        initial_projects,
        refresh_receiver,
        |inventory| {
            let refresh =
                reconcile_switch_inventory(&mut context.state, &inventory, jiff::Timestamp::now())?;
            context.windows = refresh.windows.clone();
            latest_live_inventory = Some(inventory);
            refresh_dirty |= refresh.changed;

            Ok(PickerRefresh {
                entries: context
                    .windows
                    .iter()
                    .map(picker_entry_from_window)
                    .collect(),
                projects: context.state.projects.clone(),
            })
        },
        command_span,
        &run_span,
    )? {
        PickerOutcome::Confirm(entry) => {
            let selection = {
                let _command_enter = command_span.enter();
                let _run_enter = run_span.enter();
                let selection_span = info_span!(
                    "cli.resolve_selection",
                    window_id = entry.window_id.as_str()
                );
                let _selection_enter = selection_span.enter();
                context
                    .windows
                    .iter()
                    .find(|window| window.window_id == entry.window_id)
                    .ok_or_else(|| {
                        Report::new(AppError::Tui)
                            .attach("Selected picker row no longer matches a Ghostty window")
                    })?
                    .clone()
            };

            let _command_enter = command_span.enter();
            let _run_enter = run_span.enter();
            complete_switch(
                ghostty,
                state_store,
                &mut context.state,
                &selection,
                latest_live_inventory.as_ref(),
                refresh_dirty,
            )
        }
        PickerOutcome::Cancel => {
            let _command_enter = command_span.enter();
            let _run_enter = run_span.enter();
            let cancel_span = info_span!("cli.switch_cancelled");
            let _cancel_enter = cancel_span.enter();
            if refresh_dirty {
                state_store.save(&context.state).attach(
                    "Failed to persist Ghostty session state after live refresh during switch",
                )?;
            }
            Ok(())
        }
    }
}

fn write_stdout(rendered: &str) -> Result<(), Report<AppError>> {
    let span = info_span!("cli.write_stdout", bytes = rendered.len());
    let _enter = span.enter();
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{rendered}")
        .change_context(AppError::Output)
        .attach("Failed to write command output to stdout")?;
    Ok(())
}

fn picker_entry_from_window(window: &SwitchWindow) -> PickerEntry {
    PickerEntry {
        project_key: window
            .project_path
            .as_ref()
            .map(|project_path| project_path.display().to_string())
            .unwrap_or_else(|| window.window_id.clone()),
        window_id: window.window_id.clone(),
        title: window.title.clone(),
        detail: window.detail.clone(),
    }
}
