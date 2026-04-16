use std::path::Path;

use tracing::info_span;

use crate::application::ListedWindows;

pub fn render_inventory(inventory: &ListedWindows) -> String {
    let span = info_span!(
        "cli.render_table_inventory",
        windows = inventory.windows.len()
    );
    let _enter = span.enter();
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

    use super::render_inventory;
    use crate::application::{ListedTab, ListedTerminal, ListedWindow, ListedWindows};

    #[test]
    fn renders_table_headers_and_values() {
        let inventory = sample_inventory();

        let rendered = render_inventory(&inventory);

        assert!(rendered.contains("WINDOW ID"));
        assert!(rendered.contains("PROJECT PATH"));
        assert!(rendered.contains("window-1"));
        assert!(rendered.contains("/Users/example/project"));
    }

    #[test]
    fn renders_missing_values_as_dash() {
        let inventory = ListedWindows {
            windows: vec![ListedWindow {
                window_id: "window-1".to_owned(),
                window_name: None,
                project_path: None,
                tabs: vec![ListedTab {
                    tab_id: "tab-1".to_owned(),
                    tab_name: None,
                    index: 1,
                    terminals: vec![ListedTerminal {
                        terminal_id: "terminal-1".to_owned(),
                        working_directory: None,
                    }],
                }],
                state: None,
            }],
        };

        let rendered = render_inventory(&inventory);

        assert!(rendered.contains("-"));
    }

    #[test]
    fn renders_table_in_window_order() {
        let inventory = ListedWindows {
            windows: vec![
                ListedWindow {
                    window_id: "window-2".to_owned(),
                    window_name: Some("Second".to_owned()),
                    project_path: Some(PathBuf::from("/Users/example/project-b")),
                    tabs: vec![ListedTab {
                        tab_id: "tab-2".to_owned(),
                        tab_name: Some("Shell".to_owned()),
                        index: 1,
                        terminals: vec![ListedTerminal {
                            terminal_id: "terminal-2".to_owned(),
                            working_directory: Some(PathBuf::from("/Users/example/project-b")),
                        }],
                    }],
                    state: None,
                },
                ListedWindow {
                    window_id: "window-1".to_owned(),
                    window_name: Some("First".to_owned()),
                    project_path: Some(PathBuf::from("/Users/example/project-a")),
                    tabs: vec![ListedTab {
                        tab_id: "tab-1".to_owned(),
                        tab_name: Some("Editor".to_owned()),
                        index: 1,
                        terminals: vec![ListedTerminal {
                            terminal_id: "terminal-1".to_owned(),
                            working_directory: Some(PathBuf::from("/Users/example/project-a")),
                        }],
                    }],
                    state: None,
                },
            ],
        };

        let rendered = render_inventory(&inventory);
        let window_2_index = rendered
            .find("window-2")
            .expect("window-2 row should render");
        let window_1_index = rendered
            .find("window-1")
            .expect("window-1 row should render");

        assert!(window_2_index < window_1_index);
    }

    #[test]
    fn aligns_columns_using_widest_values() {
        let inventory = ListedWindows {
            windows: vec![
                ListedWindow {
                    window_id: "w1".to_owned(),
                    window_name: Some("Short".to_owned()),
                    project_path: Some(PathBuf::from("/Users/example/a")),
                    tabs: vec![ListedTab {
                        tab_id: "tab-1".to_owned(),
                        tab_name: Some("Editor".to_owned()),
                        index: 1,
                        terminals: vec![ListedTerminal {
                            terminal_id: "terminal-1".to_owned(),
                            working_directory: Some(PathBuf::from("/Users/example/a")),
                        }],
                    }],
                    state: None,
                },
                ListedWindow {
                    window_id: "window-with-a-longer-id".to_owned(),
                    window_name: Some("A much longer window name".to_owned()),
                    project_path: Some(PathBuf::from("/Users/example/very/long/project/path")),
                    tabs: vec![ListedTab {
                        tab_id: "tab-2".to_owned(),
                        tab_name: Some("Shell".to_owned()),
                        index: 1,
                        terminals: vec![ListedTerminal {
                            terminal_id: "terminal-2".to_owned(),
                            working_directory: Some(PathBuf::from(
                                "/Users/example/very/long/project/path",
                            )),
                        }],
                    }],
                    state: None,
                },
            ],
        };

        let rendered = render_inventory(&inventory);
        let lines: Vec<&str> = rendered.lines().collect();

        let header_project_index = lines[0]
            .find("PROJECT PATH")
            .expect("header should include project path column");
        let first_row_project_index = lines[1]
            .find("/Users/example/a")
            .expect("first row should include project path");
        let second_row_project_index = lines[2]
            .find("/Users/example/very/long/project/path")
            .expect("second row should include project path");

        assert_eq!(first_row_project_index, second_row_project_index);
        assert_eq!(header_project_index, second_row_project_index);
    }

    fn sample_inventory() -> ListedWindows {
        ListedWindows {
            windows: vec![ListedWindow {
                window_id: "window-1".to_owned(),
                window_name: Some("Workspace".to_owned()),
                project_path: Some(PathBuf::from("/Users/example/project")),
                tabs: vec![ListedTab {
                    tab_id: "tab-1".to_owned(),
                    tab_name: Some("Editor".to_owned()),
                    index: 1,
                    terminals: vec![ListedTerminal {
                        terminal_id: "terminal-1".to_owned(),
                        working_directory: Some(PathBuf::from("/Users/example/project")),
                    }],
                }],
                state: None,
            }],
        }
    }
}
