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
    let inventory = ghostty
        .query_windows()
        .change_context(AppError::Ghostty)
        .attach("Failed to query Ghostty windows")?;
    let mut state = state_store
        .load()
        .attach("Failed to load persisted Ghostty session state")?;
    let observed_at = Timestamp::now();

    if state.refresh_from_inventory(&inventory, observed_at)? {
        state_store
            .save(&state)
            .attach("Failed to persist refreshed Ghostty session state after listing windows")?;
    }

    Ok(ListedWindows::from_live_and_state(&inventory, &state))
}

pub fn prepare_switch(
    ghostty: &GhosttyClient,
    state_store: &StateStore,
) -> Result<SwitchContext, Report<AppError>> {
    let span = info_span!("application.prepare_switch");
    let _enter = span.enter();
    prepare_switch_with_inventory_loader(state_store, || {
        ghostty
            .query_windows()
            .change_context(AppError::Ghostty)
            .attach("Failed to query Ghostty windows")
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
    pub window_name: Option<String>,
    pub project_path: Option<PathBuf>,
    pub title: String,
    pub detail: String,
}

fn prepare_switch_with_inventory_loader<F>(
    state_store: &StateStore,
    load_inventory: F,
) -> Result<SwitchContext, Report<AppError>>
where
    F: FnOnce() -> Result<WindowInventory, Report<AppError>>,
{
    let mut state = state_store
        .load()
        .attach("Failed to load persisted Ghostty session state")?;

    if state.projects.is_empty() {
        let inventory = load_inventory()?;
        let observed_at = Timestamp::now();

        if state.refresh_from_inventory(&inventory, observed_at)? {
            state_store.save(&state).attach(
                "Failed to persist refreshed Ghostty session state after seeding switch cache",
            )?;
        }
    }

    let windows = switch_windows_from_state(&state);
    if windows.is_empty() {
        return Err(Report::new(AppError::Ghostty).attach(
            "Ghostty returned no windows to switch to; open a Ghostty window and try again",
        ));
    }

    Ok(SwitchContext { windows, state })
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

fn switch_windows_from_state(state: &StateFile) -> Vec<SwitchWindow> {
    let mut rows: Vec<(&str, &ProjectStateRecord)> = state
        .projects
        .iter()
        .map(|(project_key, project_state)| (project_key.as_str(), project_state))
        .collect();

    rows.sort_by(|(left_key, left_record), (right_key, right_record)| {
        right_record
            .last_accessed_at
            .cmp(&left_record.last_accessed_at)
            .then_with(|| left_key.cmp(right_key))
    });

    rows.into_iter()
        .map(|(project_key, project_state)| {
            switch_window_from_project_state(project_key, project_state)
        })
        .collect()
}

fn switch_window_from_project_state(
    project_key: &str,
    project_state: &ProjectStateRecord,
) -> SwitchWindow {
    let project_path = PathBuf::from(project_key);

    SwitchWindow {
        window_id: project_state.last_window_id.clone(),
        window_name: project_state.last_window_name.clone(),
        title: switch_title_from_project_path(&project_path),
        detail: switch_detail_from_project_state(
            project_key,
            project_state.last_window_name.as_deref(),
            &project_state.last_window_id,
        ),
        project_path: Some(project_path),
    }
}

fn switch_title_from_project_path(project_path: &Path) -> String {
    project_path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| project_path.display().to_string())
}

fn switch_detail_from_project_state(
    project_key: &str,
    window_name: Option<&str>,
    window_id: &str,
) -> String {
    match window_name {
        Some(window_name) => format!("{project_key} | {window_name} | {window_id}"),
        None => format!("{project_key} | {window_id}"),
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

    state.record_project_access(
        project_path,
        &selection.window_id,
        selection.window_name.as_deref(),
        selected_at,
    )
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

    use super::{
        ListedWindows, SwitchWindow, prepare_switch_with_inventory_loader, record_switch_selection,
        switch_window_from_project_state, switch_windows_from_state,
    };
    use crate::domain::{Tab, Terminal, Window, WindowInventory};
    use crate::state::{ProjectStateRecord, StateFile, StateStore};

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
                    last_seen_at: parse_timestamp("2026-04-15T12:05:10Z"),
                    last_window_id: "window-1".to_owned(),
                    last_window_name: Some("Workspace".to_owned()),
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
                    last_seen_at: parse_timestamp("2026-04-15T12:05:10Z"),
                    last_window_id: "window-1".to_owned(),
                    last_window_name: Some("Workspace".to_owned()),
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
                    last_seen_at: parse_timestamp("2026-04-15T12:05:10Z"),
                    last_window_id: "window-1".to_owned(),
                    last_window_name: Some("Workspace".to_owned()),
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
            window_name: Some("Workspace".to_owned()),
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
                last_seen_at: selected_at,
                last_window_id: "window-1".to_owned(),
                last_window_name: Some("Workspace".to_owned()),
            })
        );
    }

    #[test]
    fn recording_pathless_switch_selection_leaves_state_unchanged() {
        let selection = SwitchWindow {
            window_id: "window-2".to_owned(),
            window_name: Some("Detached".to_owned()),
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

    #[test]
    fn listed_windows_exposes_extended_state_fields() {
        let temp_dir = unique_test_dir();
        let project_dir = temp_dir.join("project");
        fs::create_dir_all(&project_dir).expect("project dir should exist");

        let inventory = WindowInventory::from_windows(vec![Window {
            window_id: "window-1".to_owned(),
            window_name: Some("Workspace".to_owned()),
            project_path: Some(project_dir.clone()),
            tabs: vec![Tab {
                tab_id: "tab-1".to_owned(),
                tab_name: Some("Editor".to_owned()),
                index: 1,
                terminals: vec![Terminal {
                    terminal_id: "terminal-1".to_owned(),
                    working_directory: Some(project_dir.clone()),
                }],
            }],
        }]);
        let state = StateFile {
            version: 1,
            projects: BTreeMap::from([(
                project_dir
                    .canonicalize()
                    .expect("project dir should canonicalize")
                    .display()
                    .to_string(),
                ProjectStateRecord {
                    last_accessed_at: parse_timestamp("2026-04-15T12:00:00Z"),
                    last_seen_at: parse_timestamp("2026-04-15T12:05:10Z"),
                    last_window_id: "window-1".to_owned(),
                    last_window_name: Some("Workspace".to_owned()),
                },
            )]),
        };

        let listed = ListedWindows::from_live_and_state(&inventory, &state);
        let joined_state = listed.windows[0]
            .state
            .as_ref()
            .expect("window state should be joined");

        assert_eq!(joined_state.last_window_id, "window-1");
        assert_eq!(joined_state.last_window_name.as_deref(), Some("Workspace"));
        assert_eq!(
            joined_state.last_seen_at,
            parse_timestamp("2026-04-15T12:05:10Z")
        );
    }

    #[test]
    fn builds_cached_switch_rows_without_live_inventory() {
        let state = StateFile {
            version: 1,
            projects: BTreeMap::from([(
                "/tmp/project-alpha".to_owned(),
                ProjectStateRecord {
                    last_accessed_at: parse_timestamp("2026-04-16T10:00:00Z"),
                    last_seen_at: parse_timestamp("2026-04-16T10:05:00Z"),
                    last_window_id: "window-1".to_owned(),
                    last_window_name: Some("Workspace".to_owned()),
                },
            )]),
        };

        let windows = switch_windows_from_state(&state);

        assert_eq!(
            windows,
            vec![SwitchWindow {
                window_id: "window-1".to_owned(),
                window_name: Some("Workspace".to_owned()),
                project_path: Some(PathBuf::from("/tmp/project-alpha")),
                title: "project-alpha".to_owned(),
                detail: "/tmp/project-alpha | Workspace | window-1".to_owned(),
            }]
        );
    }

    #[test]
    fn cached_switch_rows_are_sorted_by_mru_descending() {
        let state = StateFile {
            version: 1,
            projects: BTreeMap::from([
                (
                    "/tmp/project-a".to_owned(),
                    ProjectStateRecord {
                        last_accessed_at: parse_timestamp("2026-04-16T09:00:00Z"),
                        last_seen_at: parse_timestamp("2026-04-16T09:05:00Z"),
                        last_window_id: "window-a".to_owned(),
                        last_window_name: Some("Workspace A".to_owned()),
                    },
                ),
                (
                    "/tmp/project-b".to_owned(),
                    ProjectStateRecord {
                        last_accessed_at: parse_timestamp("2026-04-16T11:00:00Z"),
                        last_seen_at: parse_timestamp("2026-04-16T11:05:00Z"),
                        last_window_id: "window-b".to_owned(),
                        last_window_name: Some("Workspace B".to_owned()),
                    },
                ),
            ]),
        };

        let windows = switch_windows_from_state(&state);

        assert_eq!(
            windows
                .iter()
                .map(|window| {
                    window
                        .project_path
                        .as_ref()
                        .expect("cached row path")
                        .display()
                        .to_string()
                })
                .collect::<Vec<_>>(),
            vec!["/tmp/project-b".to_owned(), "/tmp/project-a".to_owned()]
        );
    }

    #[test]
    fn cached_switch_rows_tie_break_by_project_key() {
        let accessed_at = parse_timestamp("2026-04-16T09:00:00Z");
        let state = StateFile {
            version: 1,
            projects: BTreeMap::from([
                (
                    "/tmp/project-b".to_owned(),
                    ProjectStateRecord {
                        last_accessed_at: accessed_at,
                        last_seen_at: accessed_at,
                        last_window_id: "window-b".to_owned(),
                        last_window_name: Some("Workspace B".to_owned()),
                    },
                ),
                (
                    "/tmp/project-a".to_owned(),
                    ProjectStateRecord {
                        last_accessed_at: accessed_at,
                        last_seen_at: accessed_at,
                        last_window_id: "window-a".to_owned(),
                        last_window_name: Some("Workspace A".to_owned()),
                    },
                ),
            ]),
        };

        let windows = switch_windows_from_state(&state);

        assert_eq!(
            windows
                .iter()
                .map(|window| {
                    window
                        .project_path
                        .as_ref()
                        .expect("cached row path")
                        .display()
                        .to_string()
                })
                .collect::<Vec<_>>(),
            vec!["/tmp/project-a".to_owned(), "/tmp/project-b".to_owned()]
        );
    }

    #[test]
    fn cached_switch_row_omits_window_name_segment_when_missing() {
        let window = switch_window_from_project_state(
            "/tmp/project-alpha",
            &ProjectStateRecord {
                last_accessed_at: parse_timestamp("2026-04-16T10:00:00Z"),
                last_seen_at: parse_timestamp("2026-04-16T10:05:00Z"),
                last_window_id: "window-1".to_owned(),
                last_window_name: None,
            },
        );

        assert_eq!(window.title, "project-alpha");
        assert_eq!(window.detail, "/tmp/project-alpha | window-1");
    }

    #[test]
    fn prepare_switch_uses_cached_state_without_live_query() {
        let temp_dir = unique_test_dir();
        let state_path = temp_dir.join("state.json");
        let store = StateStore::from_path(&state_path);
        let state = StateFile {
            version: 1,
            projects: BTreeMap::from([(
                "/tmp/project-alpha".to_owned(),
                ProjectStateRecord {
                    last_accessed_at: parse_timestamp("2026-04-16T10:00:00Z"),
                    last_seen_at: parse_timestamp("2026-04-16T10:05:00Z"),
                    last_window_id: "window-1".to_owned(),
                    last_window_name: Some("Workspace".to_owned()),
                },
            )]),
        };
        store.save(&state).expect("state should save");

        let context = prepare_switch_with_inventory_loader(&store, || {
            panic!("live Ghostty query should not run when cached state is populated")
        })
        .expect("cached switch context should build");

        assert_eq!(context.state, state);
        assert_eq!(context.windows, switch_windows_from_state(&context.state));
    }

    #[test]
    fn prepare_switch_seeds_from_live_inventory_when_state_is_empty() {
        let temp_dir = unique_test_dir();
        let store = StateStore::from_path(temp_dir.join("state.json"));
        let project_dir = temp_dir.join("project");
        fs::create_dir_all(&project_dir).expect("project dir should exist");

        let inventory = WindowInventory::from_windows(vec![Window {
            window_id: "window-1".to_owned(),
            window_name: Some("Workspace".to_owned()),
            project_path: Some(project_dir.clone()),
            tabs: vec![Tab {
                tab_id: "tab-1".to_owned(),
                tab_name: Some("Editor".to_owned()),
                index: 1,
                terminals: vec![Terminal {
                    terminal_id: "terminal-1".to_owned(),
                    working_directory: Some(project_dir.clone()),
                }],
            }],
        }]);

        let context = prepare_switch_with_inventory_loader(&store, || Ok(inventory.clone()))
            .expect("seeded switch context should build");
        let persisted = store.load().expect("seeded state should load");

        assert_eq!(context.state, persisted);
        assert_eq!(context.windows, switch_windows_from_state(&context.state));
        assert_eq!(context.windows.len(), 1);
        assert_eq!(
            context.windows[0].project_path.as_deref(),
            Some(
                project_dir
                    .canonicalize()
                    .expect("project dir should canonicalize")
                    .as_path()
            )
        );
    }

    #[test]
    fn prepare_switch_returns_no_windows_error_when_seeded_state_is_still_empty() {
        let temp_dir = unique_test_dir();
        let store = StateStore::from_path(temp_dir.join("state.json"));

        let report = prepare_switch_with_inventory_loader(&store, || {
            Ok(WindowInventory::from_windows(vec![]))
        })
        .expect_err("empty seed should fail");

        let rendered = format!("{report:?}");
        assert!(rendered.contains("Ghostty returned no windows to switch to"));
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
