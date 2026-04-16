use std::path::PathBuf;
use std::process::Command;

use error_stack::{Report, ResultExt};

use crate::domain::{Tab, Terminal, Window, WindowInventory};
use crate::error::AppError;

const EXPECTED_FIELD_COUNT: usize = 7;

const QUERY_WINDOWS_SCRIPT: &str = r#"
if not (application "Ghostty" is running) then
    error "Ghostty is not running" number 1001
end if

on sanitize_text(value)
    if value is missing value then
        return ""
    end if

    set normalized to value as text
    set normalized to my replace_text(normalized, return, " ")
    set normalized to my replace_text(normalized, linefeed, " ")
    set normalized to my replace_text(normalized, (character id 9), " ")
    return normalized
end sanitize_text

on replace_text(subject, search_text, replacement_text)
    set AppleScript's text item delimiters to search_text
    set text_items to every text item of subject
    set AppleScript's text item delimiters to replacement_text
    set joined_text to text_items as text
    set AppleScript's text item delimiters to ""
    return joined_text
end replace_text

set row_delimiter to linefeed
set field_delimiter to (character id 9)
set rows to {}

tell application "Ghostty"
    repeat with current_window in every window
        set window_id to my sanitize_text(id of current_window)
        set window_name to my sanitize_text(name of current_window)

        repeat with current_tab in every tab of current_window
            set tab_id to my sanitize_text(id of current_tab)
            set tab_index to (index of current_tab) as text
            set tab_name to my sanitize_text(name of current_tab)

            repeat with current_terminal in every terminal of current_tab
                set terminal_id to my sanitize_text(id of current_terminal)
                set working_directory to ""

                try
                    set working_directory to my sanitize_text(working directory of current_terminal)
                end try

                set end of rows to (window_id & field_delimiter & window_name & field_delimiter & tab_id & field_delimiter & tab_index & field_delimiter & tab_name & field_delimiter & terminal_id & field_delimiter & working_directory)
            end repeat
        end repeat
    end repeat
end tell

if (count of rows) is 0 then
    return ""
end if

set AppleScript's text item delimiters to row_delimiter
set output_text to rows as text
set AppleScript's text item delimiters to ""
return output_text
"#;

#[derive(Debug, Clone)]
pub struct GhosttyClient {
    verbose: bool,
}

impl GhosttyClient {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    pub fn query_windows(&self) -> Result<WindowInventory, Report<AppError>> {
        let stdout = self.run_query_script("query_windows", QUERY_WINDOWS_SCRIPT)?;
        parse_window_inventory(&stdout)
    }

    fn run_query_script(
        &self,
        action_name: &str,
        script: &str,
    ) -> Result<String, Report<AppError>> {
        if self.verbose {
            eprintln!("Running Ghostty AppleScript action: {action_name}");
        }

        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .change_context(AppError::AppleScript)
            .attach_with(|| format!("Failed to spawn osascript for Ghostty action: {action_name}"))
            .attach_with(|| format!("script={script}"))?;

        process_osascript_output(self.verbose, action_name, script, output)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedRow {
    window_id: String,
    window_name: Option<String>,
    tab_id: String,
    tab_index: usize,
    tab_name: Option<String>,
    terminal_id: String,
    working_directory: Option<PathBuf>,
}

fn parse_window_inventory(stdout: &str) -> Result<WindowInventory, Report<AppError>> {
    let mut windows: Vec<Window> = Vec::new();

    for (row_index, line) in stdout.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let parsed_row = parse_row(line, row_index + 1)?;
        insert_row(&mut windows, parsed_row);
    }

    Ok(WindowInventory::from_windows(windows))
}

fn parse_row(line: &str, row_number: usize) -> Result<ParsedRow, Report<AppError>> {
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() != EXPECTED_FIELD_COUNT {
        return Err(Report::new(AppError::Parse)
            .attach(format!(
                "Expected {EXPECTED_FIELD_COUNT} TSV fields, found {}",
                fields.len()
            ))
            .attach(format!("row={row_number}"))
            .attach(format!("line={line}")));
    }

    let tab_index = fields[3]
        .parse::<usize>()
        .change_context(AppError::Parse)
        .attach(format!("row={row_number}"))
        .attach("field=tab_index")
        .attach(format!("value={}", fields[3]))?;

    Ok(ParsedRow {
        window_id: fields[0].to_owned(),
        window_name: optional_text(fields[1]),
        tab_id: fields[2].to_owned(),
        tab_index,
        tab_name: optional_text(fields[4]),
        terminal_id: fields[5].to_owned(),
        working_directory: optional_path(fields[6]),
    })
}

fn optional_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn optional_path(value: &str) -> Option<PathBuf> {
    optional_text(value).map(PathBuf::from)
}

fn insert_row(windows: &mut Vec<Window>, row: ParsedRow) {
    let window_index = windows
        .iter()
        .position(|window| window.window_id == row.window_id)
        .unwrap_or_else(|| {
            windows.push(Window {
                window_id: row.window_id.clone(),
                window_name: row.window_name.clone(),
                project_path: None,
                tabs: Vec::new(),
            });
            windows.len() - 1
        });

    let window = &mut windows[window_index];
    let tab_index = window
        .tabs
        .iter()
        .position(|tab| tab.tab_id == row.tab_id)
        .unwrap_or_else(|| {
            window.tabs.push(Tab {
                tab_id: row.tab_id.clone(),
                tab_name: row.tab_name.clone(),
                index: row.tab_index,
                terminals: Vec::new(),
            });
            window.tabs.len() - 1
        });

    let tab = &mut window.tabs[tab_index];
    tab.terminals.push(Terminal {
        terminal_id: row.terminal_id,
        working_directory: row.working_directory,
    });
}

#[cfg(test)]
mod tests {
    use std::os::unix::process::ExitStatusExt;
    use std::path::PathBuf;
    use std::process::Output;

    use super::{AppError, GhosttyClient, parse_row, parse_window_inventory};

    #[test]
    fn parses_single_window_single_tab_single_terminal() {
        let inventory = parse_window_inventory(
            "window-1\tWorkspace\ttab-1\t1\tEditor\tterminal-1\t/Users/example/project\n",
        )
        .expect("inventory should parse");

        assert_eq!(inventory.windows.len(), 1);
        let window = &inventory.windows[0];
        assert_eq!(window.window_id, "window-1");
        assert_eq!(window.window_name.as_deref(), Some("Workspace"));
        assert_eq!(
            window.project_path.as_deref(),
            Some(PathBuf::from("/Users/example/project").as_path())
        );
        assert_eq!(window.tabs.len(), 1);
        assert_eq!(window.tabs[0].terminals.len(), 1);
    }

    #[test]
    fn parses_multiple_windows_and_tabs() {
        let inventory = parse_window_inventory(concat!(
            "window-1\tWorkspace\ttab-2\t2\tShell\tterminal-2\t/Users/example/project-b\n",
            "window-1\tWorkspace\ttab-1\t1\tEditor\tterminal-1\t/Users/example/project-a\n",
            "window-2\tOther\ttab-3\t1\tDocs\tterminal-3\t/Users/example/project-c\n",
        ))
        .expect("inventory should parse");

        assert_eq!(inventory.windows.len(), 2);
        assert_eq!(inventory.windows[0].tabs[0].tab_id, "tab-1");
        assert_eq!(inventory.windows[0].tabs[1].tab_id, "tab-2");
        assert_eq!(
            inventory.windows[0].project_path.as_deref(),
            Some(PathBuf::from("/Users/example/project-a").as_path())
        );
        assert_eq!(
            inventory.windows[1].project_path.as_deref(),
            Some(PathBuf::from("/Users/example/project-c").as_path())
        );
    }

    #[test]
    fn blank_optional_fields_are_none() {
        let inventory =
            parse_window_inventory("window-1\t\ttab-1\t1\t\tterminal-1\t\n").expect("parses");

        let window = &inventory.windows[0];
        assert_eq!(window.window_name, None);
        assert_eq!(window.tabs[0].tab_name, None);
        assert_eq!(window.project_path, None);
    }

    #[test]
    fn later_tabs_do_not_override_first_tab_project_path() {
        let inventory = parse_window_inventory(concat!(
            "window-1\tWorkspace\ttab-2\t2\tShell\tterminal-2\t/Users/example/project-b\n",
            "window-1\tWorkspace\ttab-1\t1\tEditor\tterminal-1\t/Users/example/project-a\n",
        ))
        .expect("inventory should parse");

        assert_eq!(
            inventory.windows[0].project_path.as_deref(),
            Some(PathBuf::from("/Users/example/project-a").as_path())
        );
    }

    #[test]
    fn rejects_malformed_column_count() {
        let report = parse_window_inventory("window-1\tWorkspace\ttab-1\t1\tEditor\tterminal-1\n")
            .expect_err("row should be rejected");

        let rendered = format!("{report:?}");
        assert!(rendered.contains("Expected 7 TSV fields"));
        assert!(rendered.contains("row=1"));
    }

    #[test]
    fn rejects_invalid_numeric_ids() {
        let report = parse_row(
            "window-1\tWorkspace\ttab-1\tnot-a-number\tEditor\tterminal-1\t/Users/example",
            3,
        )
        .expect_err("tab index should fail");

        let rendered = format!("{report:?}");
        assert!(rendered.contains("field=tab_index"));
        assert!(rendered.contains("row=3"));
        assert!(rendered.contains("value=not-a-number"));
    }

    #[test]
    fn applescript_failure_includes_status_and_stderr() {
        let report = GhosttyClient::build_osascript_error_for_test(
            "query_windows",
            "script body",
            Output {
                status: std::process::ExitStatus::from_raw(256),
                stdout: Vec::new(),
                stderr: b"permission denied".to_vec(),
            },
        )
        .expect_err("should fail");

        let rendered = format!("{report:?}");
        assert!(rendered.contains("query_windows"));
        assert!(rendered.contains("permission denied"));
        assert!(rendered.contains("status=exit status: 1"));
    }

    #[test]
    fn empty_stdout_produces_empty_inventory() {
        let inventory = parse_window_inventory("").expect("empty inventory is valid");
        assert!(inventory.windows.is_empty());
    }

    impl GhosttyClient {
        fn build_osascript_error_for_test(
            action_name: &str,
            script: &str,
            output: Output,
        ) -> Result<String, error_stack::Report<AppError>> {
            Self { verbose: false }.process_output_for_test(action_name, script, output)
        }

        fn process_output_for_test(
            &self,
            action_name: &str,
            script: &str,
            output: Output,
        ) -> Result<String, error_stack::Report<AppError>> {
            super::process_osascript_output(self.verbose, action_name, script, output)
        }
    }
}

fn process_osascript_output(
    verbose: bool,
    action_name: &str,
    script: &str,
    output: std::process::Output,
) -> Result<String, Report<AppError>> {
    let stdout = String::from_utf8(output.stdout)
        .change_context(AppError::AppleScript)
        .attach_with(|| format!("Ghostty action {action_name} produced non-UTF-8 stdout"))?;

    let stderr = String::from_utf8(output.stderr)
        .change_context(AppError::AppleScript)
        .attach_with(|| format!("Ghostty action {action_name} produced non-UTF-8 stderr"))?;

    if verbose && !stderr.trim().is_empty() {
        eprintln!("osascript stderr for {action_name}: {}", stderr.trim());
    }

    if !output.status.success() {
        return Err(Report::new(AppError::AppleScript)
            .attach(format!("Ghostty AppleScript action {action_name} failed"))
            .attach(format!("status={}", output.status))
            .attach(format!("stderr={}", stderr.trim()))
            .attach(format!("script={script}")));
    }

    Ok(stdout)
}
