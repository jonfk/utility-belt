use std::path::{Path, PathBuf};

use error_stack::{Report, ResultExt};
use jiff::Timestamp;
use serde::Serialize;
use tracing::info_span;

use crate::domain::{Tab, Terminal, Window, WindowInventory};
use crate::error::AppError;
use crate::ghostty::GhosttyClient;
use crate::state::{ProjectStateRecord, StateFile, StateStore};

pub fn list_windows(
    ghostty: &GhosttyClient,
    state_store: &StateStore,
) -> Result<ListedWindows, Report<AppError>> {
    let span = info_span!("application.list_windows");
    let _enter = span.enter();
    let (inventory, state) = load_inventory_and_state(ghostty, state_store)?;
    Ok(ListedWindows::from_live_and_state(&inventory, &state))
}

pub fn prepare_switch(
    ghostty: &GhosttyClient,
    state_store: &StateStore,
) -> Result<SwitchContext, Report<AppError>> {
    let span = info_span!("application.prepare_switch");
    let _enter = span.enter();
    let (inventory, state) = load_inventory_and_state(ghostty, state_store)?;
    let listed = ListedWindows::from_live_and_state(&inventory, &state);

    if listed.windows.is_empty() {
        return Err(Report::new(AppError::Ghostty).attach(
            "Ghostty returned no windows to switch to; open a Ghostty window and try again",
        ));
    }

    Ok(SwitchContext {
        windows: listed
            .windows
            .iter()
            .map(SwitchWindow::from_listed_window)
            .collect(),
        state,
    })
}

pub fn complete_switch(
    ghostty: &GhosttyClient,
    state_store: &StateStore,
    state: &mut StateFile,
    selection: &SwitchWindow,
) -> Result<(), Report<AppError>> {
    let span = info_span!(
        "application.complete_switch",
        window_id = selection.window_id.as_str(),
        has_project_path = selection.project_path.is_some()
    );
    let _enter = span.enter();
    ghostty
        .focus_window(&selection.window_id)
        .change_context(AppError::Ghostty)
        .attach_with(|| format!("Failed to focus Ghostty window {}", selection.window_id))?;

    if record_switch_selection(state, selection, Timestamp::now())? {
        state_store
            .save(state)
            .attach("Failed to persist Ghostty session state after switching windows")?;
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchContext {
    pub windows: Vec<SwitchWindow>,
    pub state: StateFile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchWindow {
    pub window_id: String,
    pub project_path: Option<PathBuf>,
    pub title: String,
    pub detail: String,
}

impl SwitchWindow {
    fn from_listed_window(window: &ListedWindow) -> Self {
        let title = switch_title(window);
        let detail = switch_detail(window);

        Self {
            window_id: window.window_id.clone(),
            project_path: window.project_path.clone(),
            title,
            detail,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListedWindows {
    pub windows: Vec<ListedWindow>,
}

impl ListedWindows {
    fn from_live_and_state(inventory: &WindowInventory, state: &StateFile) -> Self {
        Self {
            windows: inventory
                .windows
                .iter()
                .map(|window| ListedWindow::from_live_and_state(window, state))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListedWindow {
    pub window_id: String,
    pub window_name: Option<String>,
    pub project_path: Option<PathBuf>,
    pub tabs: Vec<ListedTab>,
    pub state: Option<ProjectStateRecord>,
}

impl ListedWindow {
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    fn from_live_and_state(window: &Window, state: &StateFile) -> Self {
        Self {
            window_id: window.window_id.clone(),
            window_name: window.window_name.clone(),
            project_path: window.project_path.clone(),
            tabs: window.tabs.iter().map(ListedTab::from_live).collect(),
            state: project_state_for_path(window.project_path.as_deref(), state),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListedTab {
    pub tab_id: String,
    pub tab_name: Option<String>,
    pub index: usize,
    pub terminals: Vec<ListedTerminal>,
}

impl ListedTab {
    fn from_live(tab: &Tab) -> Self {
        Self {
            tab_id: tab.tab_id.clone(),
            tab_name: tab.tab_name.clone(),
            index: tab.index,
            terminals: tab
                .terminals
                .iter()
                .map(ListedTerminal::from_live)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListedTerminal {
    pub terminal_id: String,
    pub working_directory: Option<PathBuf>,
}

impl ListedTerminal {
    fn from_live(terminal: &Terminal) -> Self {
        Self {
            terminal_id: terminal.terminal_id.clone(),
            working_directory: terminal.working_directory.clone(),
        }
    }
}

fn load_inventory_and_state(
    ghostty: &GhosttyClient,
    state_store: &StateStore,
) -> Result<(WindowInventory, StateFile), Report<AppError>> {
    let span = info_span!("application.load_inventory_and_state");
    let _enter = span.enter();
    let inventory = ghostty
        .query_windows()
        .change_context(AppError::Ghostty)
        .attach("Failed to query Ghostty windows")?;
    let state = state_store
        .load()
        .attach("Failed to load persisted Ghostty session state")?;

    Ok((inventory, state))
}

fn switch_title(window: &ListedWindow) -> String {
    if let Some(project_path) = &window.project_path {
        return project_path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| project_path.display().to_string());
    }

    window
        .window_name
        .clone()
        .unwrap_or_else(|| "No project path".to_owned())
}

fn switch_detail(window: &ListedWindow) -> String {
    match (&window.project_path, &window.window_name) {
        (Some(project_path), Some(window_name)) => format!(
            "{} | {} | {}",
            project_path.display(),
            window_name,
            window.window_id
        ),
        (Some(project_path), None) => {
            format!("{} | {}", project_path.display(), window.window_id)
        }
        (None, Some(window_name)) => {
            format!("No project path | {} | {}", window_name, window.window_id)
        }
        (None, None) => format!("No project path | {}", window.window_id),
    }
}

fn record_switch_selection(
    state: &mut StateFile,
    selection: &SwitchWindow,
    selected_at: Timestamp,
) -> Result<bool, Report<AppError>> {
    let Some(project_path) = selection.project_path.as_deref() else {
        return Ok(false);
    };

    state.record_project_access(project_path, selected_at)?;
    Ok(true)
}

fn project_state_for_path(
    project_path: Option<&Path>,
    state: &StateFile,
) -> Option<ProjectStateRecord> {
    let project_path = project_path?;
    let canonical_key = StateStore::canonical_project_key(project_path).ok()?;
    state.projects.get(&canonical_key).cloned()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use jiff::Timestamp;

    use super::{ListedWindows, SwitchWindow, record_switch_selection};
    use crate::domain::{Tab, Terminal, Window, WindowInventory};
    use crate::state::{ProjectStateRecord, StateFile};

    #[test]
    fn joins_matching_canonical_project_state() {
        let temp_dir = unique_test_dir();
        let project_dir = temp_dir.join("project");
        fs::create_dir_all(&project_dir).expect("project dir should exist");
        fs::create_dir_all(project_dir.join("subdir")).expect("subdir should exist");

        let inventory = WindowInventory::from_windows(vec![Window {
            window_id: "window-1".to_owned(),
            window_name: Some("Workspace".to_owned()),
            project_path: None,
            tabs: vec![Tab {
                tab_id: "tab-1".to_owned(),
                tab_name: Some("Editor".to_owned()),
                index: 1,
                terminals: vec![Terminal {
                    terminal_id: "terminal-1".to_owned(),
                    working_directory: Some(project_dir.join("subdir").join("..")),
                }],
            }],
        }]);

        let state = StateFile {
            version: 1,
            projects: BTreeMap::from([(
                project_dir
                    .canonicalize()
                    .expect("path should canonicalize")
                    .display()
                    .to_string(),
                ProjectStateRecord {
                    last_accessed_at: parse_timestamp("2026-04-15T12:00:00Z"),
                },
            )]),
        };

        let listed = ListedWindows::from_live_and_state(&inventory, &state);

        assert_eq!(
            listed.windows[0]
                .state
                .as_ref()
                .expect("window state should be joined")
                .last_accessed_at,
            parse_timestamp("2026-04-15T12:00:00Z")
        );
    }

    #[test]
    fn omits_state_when_project_path_cannot_be_canonicalized() {
        let inventory = WindowInventory::from_windows(vec![Window {
            window_id: "window-1".to_owned(),
            window_name: Some("Workspace".to_owned()),
            project_path: Some(PathBuf::from("/path/that/does/not/exist")),
            tabs: vec![Tab {
                tab_id: "tab-1".to_owned(),
                tab_name: Some("Editor".to_owned()),
                index: 1,
                terminals: vec![Terminal {
                    terminal_id: "terminal-1".to_owned(),
                    working_directory: Some(PathBuf::from("/path/that/does/not/exist")),
                }],
            }],
        }]);

        let state = StateFile {
            version: 1,
            projects: BTreeMap::from([(
                "/path/that/does/not/exist".to_owned(),
                ProjectStateRecord {
                    last_accessed_at: parse_timestamp("2026-04-15T12:00:00Z"),
                },
            )]),
        };

        let listed = ListedWindows::from_live_and_state(&inventory, &state);

        assert_eq!(listed.windows[0].state, None);
    }

    #[test]
    fn preserves_window_order_after_state_merge() {
        let temp_dir = unique_test_dir();
        let project_a = temp_dir.join("project-a");
        let project_b = temp_dir.join("project-b");
        fs::create_dir_all(&project_a).expect("project a should exist");
        fs::create_dir_all(&project_b).expect("project b should exist");

        let inventory = WindowInventory::from_windows(vec![
            Window {
                window_id: "window-2".to_owned(),
                window_name: Some("Second".to_owned()),
                project_path: Some(project_b.clone()),
                tabs: vec![Tab {
                    tab_id: "tab-2".to_owned(),
                    tab_name: Some("Shell".to_owned()),
                    index: 1,
                    terminals: vec![Terminal {
                        terminal_id: "terminal-2".to_owned(),
                        working_directory: Some(project_b.clone()),
                    }],
                }],
            },
            Window {
                window_id: "window-1".to_owned(),
                window_name: Some("First".to_owned()),
                project_path: Some(project_a.clone()),
                tabs: vec![Tab {
                    tab_id: "tab-1".to_owned(),
                    tab_name: Some("Editor".to_owned()),
                    index: 1,
                    terminals: vec![Terminal {
                        terminal_id: "terminal-1".to_owned(),
                        working_directory: Some(project_a.clone()),
                    }],
                }],
            },
        ]);

        let state = StateFile {
            version: 1,
            projects: BTreeMap::from([(
                project_a
                    .canonicalize()
                    .expect("project a path should canonicalize")
                    .display()
                    .to_string(),
                ProjectStateRecord {
                    last_accessed_at: parse_timestamp("2026-04-15T12:00:00Z"),
                },
            )]),
        };

        let listed = ListedWindows::from_live_and_state(&inventory, &state);

        assert_eq!(listed.windows[0].window_id, "window-2");
        assert_eq!(listed.windows[1].window_id, "window-1");
    }

    #[test]
    fn recording_switch_selection_creates_or_updates_project_state() {
        let temp_dir = unique_test_dir();
        let project_dir = temp_dir.join("project");
        fs::create_dir_all(&project_dir).expect("project dir should exist");

        let selection = SwitchWindow {
            window_id: "window-1".to_owned(),
            project_path: Some(project_dir.clone()),
            title: "project".to_owned(),
            detail: project_dir.display().to_string(),
        };
        let selected_at = parse_timestamp("2026-04-16T09:30:00Z");
        let mut state = StateFile::empty();

        let changed = record_switch_selection(&mut state, &selection, selected_at)
            .expect("recording selection should succeed");

        let key = project_dir
            .canonicalize()
            .expect("project dir should canonicalize")
            .display()
            .to_string();
        assert!(changed);
        assert_eq!(
            state.projects.get(&key),
            Some(&ProjectStateRecord {
                last_accessed_at: selected_at,
            })
        );
    }

    #[test]
    fn recording_pathless_switch_selection_leaves_state_unchanged() {
        let selection = SwitchWindow {
            window_id: "window-2".to_owned(),
            project_path: None,
            title: "No project path".to_owned(),
            detail: "window-2".to_owned(),
        };
        let selected_at = parse_timestamp("2026-04-16T09:30:00Z");
        let mut state = StateFile::empty();

        let changed = record_switch_selection(&mut state, &selection, selected_at)
            .expect("pathless selection should be ignored");

        assert!(!changed);
        assert_eq!(state, StateFile::empty());
    }

    fn unique_test_dir() -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "ghostty-session-manager-application-tests-{}-{}",
            timestamp, counter
        ));

        if dir.exists() {
            fs::remove_dir_all(&dir).expect("stale temp dir should be removable");
        }

        dir
    }

    fn parse_timestamp(input: &str) -> Timestamp {
        input.parse().expect("timestamp fixture should parse")
    }
}
