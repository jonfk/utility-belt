use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WindowInventory {
    pub windows: Vec<Window>,
}

impl WindowInventory {
    pub fn from_windows(mut windows: Vec<Window>) -> Self {
        for window in &mut windows {
            window.tabs.sort_by_key(|tab| tab.index);
            window.project_path = window.derived_project_path();
        }

        Self { windows }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Window {
    pub window_id: String,
    pub window_name: Option<String>,
    pub project_path: Option<PathBuf>,
    pub tabs: Vec<Tab>,
}

impl Window {
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    fn derived_project_path(&self) -> Option<PathBuf> {
        self.tabs
            .iter()
            .min_by_key(|tab| tab.index)
            .and_then(|tab| tab.terminals.first())
            .and_then(|terminal| terminal.working_directory.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Tab {
    pub tab_id: String,
    pub tab_name: Option<String>,
    pub index: usize,
    pub terminals: Vec<Terminal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Terminal {
    pub terminal_id: String,
    pub working_directory: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::{Tab, Terminal, Window, WindowInventory};
    use std::path::PathBuf;

    #[test]
    fn from_windows_sorts_tabs_by_index_and_preserves_window_order() {
        let inventory = WindowInventory::from_windows(vec![
            Window {
                window_id: "window-2".to_owned(),
                window_name: Some("Second".to_owned()),
                project_path: None,
                tabs: vec![Tab {
                    tab_id: "tab-2".to_owned(),
                    tab_name: Some("Later".to_owned()),
                    index: 2,
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
                tabs: vec![
                    Tab {
                        tab_id: "tab-2".to_owned(),
                        tab_name: Some("Later".to_owned()),
                        index: 2,
                        terminals: vec![Terminal {
                            terminal_id: "terminal-2".to_owned(),
                            working_directory: Some(PathBuf::from("/Users/example/project-b")),
                        }],
                    },
                    Tab {
                        tab_id: "tab-1".to_owned(),
                        tab_name: Some("Earlier".to_owned()),
                        index: 1,
                        terminals: vec![Terminal {
                            terminal_id: "terminal-1".to_owned(),
                            working_directory: Some(PathBuf::from("/Users/example/project-a")),
                        }],
                    },
                ],
            },
        ]);

        assert_eq!(inventory.windows[0].window_id, "window-2");
        assert_eq!(inventory.windows[1].window_id, "window-1");
        assert_eq!(inventory.windows[1].tabs[0].tab_id, "tab-1");
        assert_eq!(inventory.windows[1].tabs[1].tab_id, "tab-2");
    }

    #[test]
    fn from_windows_uses_first_terminal_of_lowest_index_tab_for_project_path() {
        let inventory = WindowInventory::from_windows(vec![Window {
            window_id: "window-1".to_owned(),
            window_name: Some("Workspace".to_owned()),
            project_path: Some(PathBuf::from("/should/be/replaced")),
            tabs: vec![
                Tab {
                    tab_id: "tab-2".to_owned(),
                    tab_name: Some("Later".to_owned()),
                    index: 2,
                    terminals: vec![Terminal {
                        terminal_id: "terminal-2".to_owned(),
                        working_directory: Some(PathBuf::from("/Users/example/project-b")),
                    }],
                },
                Tab {
                    tab_id: "tab-1".to_owned(),
                    tab_name: Some("Earlier".to_owned()),
                    index: 1,
                    terminals: vec![
                        Terminal {
                            terminal_id: "terminal-1".to_owned(),
                            working_directory: Some(PathBuf::from("/Users/example/project-a")),
                        },
                        Terminal {
                            terminal_id: "terminal-3".to_owned(),
                            working_directory: Some(PathBuf::from("/Users/example/project-c")),
                        },
                    ],
                },
            ],
        }]);

        assert_eq!(
            inventory.windows[0].project_path.as_deref(),
            Some(PathBuf::from("/Users/example/project-a").as_path())
        );
    }

    #[test]
    fn from_windows_does_not_fall_back_when_first_terminal_has_no_working_directory() {
        let inventory = WindowInventory::from_windows(vec![Window {
            window_id: "window-1".to_owned(),
            window_name: Some("Workspace".to_owned()),
            project_path: Some(PathBuf::from("/should/be/replaced")),
            tabs: vec![
                Tab {
                    tab_id: "tab-2".to_owned(),
                    tab_name: Some("Later".to_owned()),
                    index: 2,
                    terminals: vec![Terminal {
                        terminal_id: "terminal-2".to_owned(),
                        working_directory: Some(PathBuf::from("/Users/example/project-b")),
                    }],
                },
                Tab {
                    tab_id: "tab-1".to_owned(),
                    tab_name: Some("Earlier".to_owned()),
                    index: 1,
                    terminals: vec![
                        Terminal {
                            terminal_id: "terminal-1".to_owned(),
                            working_directory: None,
                        },
                        Terminal {
                            terminal_id: "terminal-3".to_owned(),
                            working_directory: Some(PathBuf::from("/Users/example/project-c")),
                        },
                    ],
                },
            ],
        }]);

        assert_eq!(inventory.windows[0].project_path, None);
    }
}
