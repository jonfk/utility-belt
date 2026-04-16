use std::io::{self, Write};
use std::path::{Path, PathBuf};

use error_stack::{Report, ResultExt};
use serde::Serialize;

use crate::cli::{Cli, Command};
use crate::domain::{Tab, Terminal, Window, WindowInventory};
use crate::error::AppError;
use crate::ghostty::GhosttyClient;
use crate::state::{ProjectStateRecord, StateFile, StateStore};

pub fn run(cli: Cli) -> Result<(), Report<AppError>> {
    let ghostty = GhosttyClient::new(cli.verbose);

    match cli.command {
        Command::Ls { json } => run_ls(&ghostty, json),
    }
}

fn run_ls(ghostty: &GhosttyClient, json: bool) -> Result<(), Report<AppError>> {
    let inventory = ghostty
        .query_windows()
        .change_context(AppError::Ghostty)
        .attach("Failed to query Ghostty windows")?;

    if json {
        let state = StateStore::from_default_path()
            .and_then(|store| store.load())
            .attach("Failed to load persisted Ghostty session state")?;
        let rendered = render_inventory_json(&inventory, &state)?;
        write_stdout(&rendered)?;
        return Ok(());
    }

    write_stdout(&render_inventory_table(&inventory))?;
    Ok(())
}

fn write_stdout(rendered: &str) -> Result<(), Report<AppError>> {
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{rendered}")
        .change_context(AppError::Output)
        .attach("Failed to write command output to stdout")?;
    Ok(())
}

fn render_inventory_json(
    inventory: &WindowInventory,
    state: &StateFile,
) -> Result<String, Report<AppError>> {
    let projected = JsonWindowInventory::from_live_and_state(inventory, state);

    serde_json::to_string_pretty(&projected)
        .change_context(AppError::Output)
        .attach("Failed to serialize Ghostty window inventory as JSON")
}

fn render_inventory_table(inventory: &WindowInventory) -> String {
    let headers = ["WINDOW ID", "WINDOW NAME", "PROJECT PATH", "TAB COUNT"];
    let rows: Vec<[String; 4]> = inventory
        .windows
        .iter()
        .map(|window| {
            [
                window.window_id.clone(),
                display_optional_string(window.window_name.as_deref()),
                display_optional_path(window.project_path.as_deref()),
                window.tab_count().to_string(),
            ]
        })
        .collect();

    let mut widths = [
        headers[0].len(),
        headers[1].len(),
        headers[2].len(),
        headers[3].len(),
    ];

    for row in &rows {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(cell.len());
        }
    }

    let mut lines = Vec::with_capacity(rows.len() + 1);
    lines.push(format_row(&headers, &widths));

    for row in &rows {
        let values = [&row[0], &row[1], &row[2], &row[3]];
        lines.push(format_row(&values, &widths));
    }

    lines.join("\n")
}

fn format_row(values: &[impl AsRef<str>], widths: &[usize; 4]) -> String {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| format!("{:<width$}", value.as_ref(), width = widths[index]))
        .collect::<Vec<_>>()
        .join("  ")
}

fn display_optional_string(value: Option<&str>) -> String {
    value.unwrap_or("-").to_owned()
}

fn display_optional_path(value: Option<&Path>) -> String {
    value
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "-".to_owned())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct JsonWindowInventory {
    windows: Vec<JsonWindow>,
}

impl JsonWindowInventory {
    fn from_live_and_state(inventory: &WindowInventory, state: &StateFile) -> Self {
        Self {
            windows: inventory
                .windows
                .iter()
                .map(|window| JsonWindow::from_live_and_state(window, state))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct JsonWindow {
    window_id: String,
    window_name: Option<String>,
    project_path: Option<PathBuf>,
    tabs: Vec<JsonTab>,
    state: Option<ProjectStateRecord>,
}

impl JsonWindow {
    fn from_live_and_state(window: &Window, state: &StateFile) -> Self {
        Self {
            window_id: window.window_id.clone(),
            window_name: window.window_name.clone(),
            project_path: window.project_path.clone(),
            tabs: window.tabs.iter().map(JsonTab::from_live).collect(),
            state: project_state_for_window(window, state),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct JsonTab {
    tab_id: String,
    tab_name: Option<String>,
    index: usize,
    terminals: Vec<JsonTerminal>,
}

impl JsonTab {
    fn from_live(tab: &Tab) -> Self {
        Self {
            tab_id: tab.tab_id.clone(),
            tab_name: tab.tab_name.clone(),
            index: tab.index,
            terminals: tab.terminals.iter().map(JsonTerminal::from_live).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct JsonTerminal {
    terminal_id: String,
    working_directory: Option<PathBuf>,
}

impl JsonTerminal {
    fn from_live(terminal: &Terminal) -> Self {
        Self {
            terminal_id: terminal.terminal_id.clone(),
            working_directory: terminal.working_directory.clone(),
        }
    }
}

fn project_state_for_window(window: &Window, state: &StateFile) -> Option<ProjectStateRecord> {
    let project_path = window.project_path.as_deref()?;
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
    use serde_json::Value;

    use crate::domain::{Tab, Terminal, Window, WindowInventory};
    use crate::state::{ProjectStateRecord, StateFile};

    use super::{render_inventory_json, render_inventory_table};

    #[test]
    fn renders_table_headers_and_values() {
        let inventory = WindowInventory::from_windows(vec![Window {
            window_id: "window-1".to_owned(),
            window_name: Some("Workspace".to_owned()),
            project_path: Some(PathBuf::from("/Users/example/project")),
            tabs: vec![Tab {
                tab_id: "tab-1".to_owned(),
                tab_name: Some("Editor".to_owned()),
                index: 1,
                terminals: vec![Terminal {
                    terminal_id: "terminal-1".to_owned(),
                    working_directory: Some(PathBuf::from("/Users/example/project")),
                }],
            }],
        }]);

        let rendered = render_inventory_table(&inventory);

        assert!(rendered.contains("WINDOW ID"));
        assert!(rendered.contains("PROJECT PATH"));
        assert!(rendered.contains("window-1"));
        assert!(rendered.contains("/Users/example/project"));
    }

    #[test]
    fn renders_missing_values_as_dash() {
        let inventory = WindowInventory::from_windows(vec![Window {
            window_id: "window-1".to_owned(),
            window_name: None,
            project_path: None,
            tabs: vec![Tab {
                tab_id: "tab-1".to_owned(),
                tab_name: None,
                index: 1,
                terminals: vec![Terminal {
                    terminal_id: "terminal-1".to_owned(),
                    working_directory: None,
                }],
            }],
        }]);

        let rendered = render_inventory_table(&inventory);
        assert!(rendered.contains("-"));
    }

    #[test]
    fn renders_table_in_window_insertion_order() {
        let inventory = WindowInventory::from_windows(vec![
            Window {
                window_id: "window-2".to_owned(),
                window_name: Some("Second".to_owned()),
                project_path: None,
                tabs: vec![Tab {
                    tab_id: "tab-2".to_owned(),
                    tab_name: Some("Shell".to_owned()),
                    index: 1,
                    terminals: vec![Terminal {
                        terminal_id: "terminal-2".to_owned(),
                        working_directory: Some(PathBuf::from("/Users/example/project-b")),
                    }],
                }],
            },
            Window {
                window_id: "window-1".to_owned(),
                window_name: Some("First".to_owned()),
                project_path: None,
                tabs: vec![Tab {
                    tab_id: "tab-1".to_owned(),
                    tab_name: Some("Editor".to_owned()),
                    index: 1,
                    terminals: vec![Terminal {
                        terminal_id: "terminal-1".to_owned(),
                        working_directory: Some(PathBuf::from("/Users/example/project-a")),
                    }],
                }],
            },
        ]);

        let rendered = render_inventory_table(&inventory);
        let window_2_index = rendered
            .find("window-2")
            .expect("window-2 row should render");
        let window_1_index = rendered
            .find("window-1")
            .expect("window-1 row should render");

        assert!(window_2_index < window_1_index);
    }

    #[test]
    fn renders_json_with_stable_field_names_nulls_and_sorted_tabs() {
        let inventory = WindowInventory::from_windows(vec![Window {
            window_id: "window-1".to_owned(),
            window_name: None,
            project_path: Some(PathBuf::from("/ignored/by-normalization")),
            tabs: vec![
                Tab {
                    tab_id: "tab-2".to_owned(),
                    tab_name: Some("Shell".to_owned()),
                    index: 2,
                    terminals: vec![Terminal {
                        terminal_id: "terminal-2".to_owned(),
                        working_directory: Some(PathBuf::from("/Users/example/project-b")),
                    }],
                },
                Tab {
                    tab_id: "tab-1".to_owned(),
                    tab_name: None,
                    index: 1,
                    terminals: vec![Terminal {
                        terminal_id: "terminal-1".to_owned(),
                        working_directory: None,
                    }],
                },
            ],
        }]);

        let rendered =
            render_inventory_json(&inventory, &StateFile::empty()).expect("json should render");
        let value: Value = serde_json::from_str(&rendered).expect("json should parse");

        assert!(value["windows"][0]["project_path"].is_null());
        assert_eq!(value["windows"][0]["tabs"][0]["tab_id"], "tab-1");
        assert_eq!(value["windows"][0]["tabs"][0]["index"], 1);
        assert!(value["windows"][0]["tabs"][0]["terminals"][0]["working_directory"].is_null());
        assert!(value["windows"][0]["state"].is_null());
    }

    #[test]
    fn renders_json_with_matching_canonical_project_state() {
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

        let rendered = render_inventory_json(&inventory, &state).expect("json should render");
        let value: Value = serde_json::from_str(&rendered).expect("json should parse");

        assert_eq!(
            value["windows"][0]["state"]["last_accessed_at"],
            "2026-04-15T12:00:00Z"
        );
    }

    #[test]
    fn renders_json_without_state_when_project_path_cannot_be_canonicalized() {
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

        let rendered = render_inventory_json(&inventory, &state).expect("json should render");
        let value: Value = serde_json::from_str(&rendered).expect("json should parse");

        assert!(value["windows"][0]["state"].is_null());
    }

    #[test]
    fn renders_json_preserving_window_order_after_state_merge() {
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

        let rendered = render_inventory_json(&inventory, &state).expect("json should render");
        let value: Value = serde_json::from_str(&rendered).expect("json should parse");

        assert_eq!(value["windows"][0]["window_id"], "window-2");
        assert_eq!(value["windows"][1]["window_id"], "window-1");
    }

    fn unique_test_dir() -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "ghostty-session-manager-app-tests-{}-{}",
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
