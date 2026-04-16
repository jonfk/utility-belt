use error_stack::{Report, ResultExt};

use crate::application::ListedWindows;
use crate::error::AppError;

pub fn render_inventory(inventory: &ListedWindows) -> Result<String, Report<AppError>> {
    serde_json::to_string_pretty(inventory)
        .change_context(AppError::Output)
        .attach("Failed to serialize Ghostty window inventory as JSON")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::Value;

    use super::render_inventory;
    use crate::application::{ListedTab, ListedTerminal, ListedWindow, ListedWindows};

    #[test]
    fn renders_json_with_stable_field_names_and_nulls() {
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

        let rendered = render_inventory(&inventory).expect("json should render");
        let value: Value = serde_json::from_str(&rendered).expect("json should parse");

        assert!(value["windows"][0]["window_name"].is_null());
        assert!(value["windows"][0]["project_path"].is_null());
        assert_eq!(value["windows"][0]["tabs"][0]["tab_id"], "tab-1");
        assert!(value["windows"][0]["tabs"][0]["tab_name"].is_null());
        assert!(value["windows"][0]["tabs"][0]["terminals"][0]["working_directory"].is_null());
        assert!(value["windows"][0]["state"].is_null());
    }

    #[test]
    fn preserves_tab_order_in_serialized_json() {
        let inventory = ListedWindows {
            windows: vec![ListedWindow {
                window_id: "window-1".to_owned(),
                window_name: Some("Workspace".to_owned()),
                project_path: Some(PathBuf::from("/Users/example/project")),
                tabs: vec![
                    ListedTab {
                        tab_id: "tab-1".to_owned(),
                        tab_name: Some("Editor".to_owned()),
                        index: 1,
                        terminals: vec![ListedTerminal {
                            terminal_id: "terminal-1".to_owned(),
                            working_directory: Some(PathBuf::from("/Users/example/project")),
                        }],
                    },
                    ListedTab {
                        tab_id: "tab-2".to_owned(),
                        tab_name: Some("Shell".to_owned()),
                        index: 2,
                        terminals: vec![ListedTerminal {
                            terminal_id: "terminal-2".to_owned(),
                            working_directory: Some(PathBuf::from("/Users/example/project")),
                        }],
                    },
                ],
                state: None,
            }],
        };

        let rendered = render_inventory(&inventory).expect("json should render");
        let value: Value = serde_json::from_str(&rendered).expect("json should parse");

        assert_eq!(value["windows"][0]["tabs"][0]["tab_id"], "tab-1");
        assert_eq!(value["windows"][0]["tabs"][1]["tab_id"], "tab-2");
    }
}
