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
