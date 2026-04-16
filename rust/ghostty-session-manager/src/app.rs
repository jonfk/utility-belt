use std::io::{self, Write};
use std::path::Path;

use error_stack::{Report, ResultExt};

use crate::cli::{Cli, Command};
use crate::domain::WindowInventory;
use crate::error::AppError;
use crate::ghostty::GhosttyClient;

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
        let rendered = render_inventory_json(&inventory)?;
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

fn render_inventory_json(inventory: &WindowInventory) -> Result<String, Report<AppError>> {
    serde_json::to_string_pretty(inventory)
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::Value;

    use crate::domain::{Tab, Terminal, Window, WindowInventory};

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

        let rendered = render_inventory_json(&inventory).expect("json should render");
        let value: Value = serde_json::from_str(&rendered).expect("json should parse");

        assert!(value["windows"][0]["project_path"].is_null());
        assert_eq!(value["windows"][0]["tabs"][0]["tab_id"], "tab-1");
        assert_eq!(value["windows"][0]["tabs"][0]["index"], 1);
        assert!(value["windows"][0]["tabs"][0]["terminals"][0]["working_directory"].is_null());
    }
}
