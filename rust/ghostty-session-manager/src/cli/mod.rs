mod args;
mod json;
mod table;

use std::env;
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
        let invocation_cwd = env::current_dir().ok();
        prepare_switch(ghostty, state_store, invocation_cwd.as_deref())?
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
        context.current_project_key.clone(),
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
                    project_key = entry.project_key.as_str(),
                    window_id = entry.window_id.as_str()
                );
                let _selection_enter = selection_span.enter();
                resolve_selected_window(&entry, &context.windows)?
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
        project_key: switch_window_project_key(window),
        window_id: window.window_id.clone(),
        primary_label: window.title.clone(),
        secondary_path: window
            .project_path
            .as_ref()
            .map(|project_path| project_path.display().to_string()),
        window_name: window.window_name.clone(),
    }
}

fn resolve_selected_window(
    entry: &PickerEntry,
    windows: &[SwitchWindow],
) -> Result<SwitchWindow, Report<AppError>> {
    windows
        .iter()
        .find(|window| {
            window.window_id == entry.window_id
                && switch_window_project_key(window) == entry.project_key
        })
        .cloned()
        .ok_or_else(|| {
            Report::new(AppError::Tui)
                .attach("Selected picker row no longer matches a Ghostty window")
                .attach(format!("selected_project_key={}", entry.project_key))
                .attach(format!("selected_window_id={}", entry.window_id))
        })
}

fn switch_window_project_key(window: &SwitchWindow) -> String {
    window
        .project_path
        .as_ref()
        .map(|project_path| project_path.display().to_string())
        .unwrap_or_else(|| window.window_id.clone())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{picker_entry_from_window, resolve_selected_window};
    use crate::application::SwitchWindow;
    use crate::tui::PickerEntry;

    #[test]
    fn resolve_selected_window_prefers_matching_project_key_when_window_ids_repeat() {
        let windows = vec![
            sample_window("/tmp/project-a", "window-1"),
            sample_window("/tmp/project-b", "window-1"),
        ];

        let entry = picker_entry_from_window(&windows[1]);

        let selected =
            resolve_selected_window(&entry, &windows).expect("selection should resolve exactly");

        assert_eq!(selected, windows[1]);
    }

    #[test]
    fn resolve_selected_window_requires_matching_project_key_and_window_id() {
        let windows = vec![
            sample_window("/tmp/project-a", "window-1"),
            sample_window("/tmp/project-b", "window-2"),
        ];
        let entry = PickerEntry {
            project_key: "/tmp/project-a".to_owned(),
            window_id: "window-2".to_owned(),
            primary_label: "project-a".to_owned(),
            secondary_path: Some("/tmp/project-a".to_owned()),
            window_name: Some("Workspace".to_owned()),
        };

        let error = resolve_selected_window(&entry, &windows)
            .expect_err("mismatched identity tuple should not resolve");
        let rendered = format!("{error:?}");

        assert!(rendered.contains("selected_project_key=/tmp/project-a"));
        assert!(rendered.contains("selected_window_id=window-2"));
    }

    fn sample_window(project_key: &str, window_id: &str) -> SwitchWindow {
        SwitchWindow {
            window_id: window_id.to_owned(),
            window_name: Some("Workspace".to_owned()),
            project_path: Some(PathBuf::from(project_key)),
            title: PathBuf::from(project_key)
                .file_name()
                .expect("sample project should have a basename")
                .to_string_lossy()
                .into_owned(),
            detail: format!("{project_key} | Workspace | {window_id}"),
        }
    }
}
