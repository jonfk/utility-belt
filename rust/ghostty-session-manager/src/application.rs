use std::path::{Path, PathBuf};

use error_stack::{Report, ResultExt};
use serde::Serialize;

use crate::domain::{Tab, Terminal, Window, WindowInventory};
use crate::error::AppError;
use crate::ghostty::GhosttyClient;
use crate::state::{ProjectStateRecord, StateFile, StateStore};

pub fn list_windows(
    ghostty: &GhosttyClient,
    state_store: &StateStore,
) -> Result<ListedWindows, Report<AppError>> {
    let inventory = ghostty
        .query_windows()
        .change_context(AppError::Ghostty)
        .attach("Failed to query Ghostty windows")?;
    let state = state_store
        .load()
        .attach("Failed to load persisted Ghostty session state")?;

    Ok(ListedWindows::from_live_and_state(&inventory, &state))
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
            terminals: tab.terminals.iter().map(ListedTerminal::from_live).collect(),
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

fn project_state_for_path(project_path: Option<&Path>, state: &StateFile) -> Option<ProjectStateRecord> {
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

    use super::ListedWindows;
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
