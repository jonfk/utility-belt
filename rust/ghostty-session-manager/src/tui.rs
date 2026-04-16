use std::collections::BTreeMap;
use std::io;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use error_stack::{Report, ResultExt};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use tracing::info_span;

use crate::error::AppError;
use crate::search::rank_project_keys;
use crate::state::ProjectStateRecord;

#[derive(Debug)]
pub enum RefreshMessage {
    Success(crate::domain::WindowInventory),
    Failure(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerEntry {
    pub project_key: String,
    pub window_id: String,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerOutcome {
    Confirm(PickerEntry),
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerRefresh {
    pub entries: Vec<PickerEntry>,
    pub projects: BTreeMap<String, ProjectStateRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerState {
    projects: BTreeMap<String, ProjectStateRecord>,
    entries_by_project_key: BTreeMap<String, PickerEntry>,
    filtered_project_keys: Vec<String>,
    selected_project_key: Option<String>,
    query: String,
}

impl PickerState {
    pub fn new(entries: Vec<PickerEntry>, projects: BTreeMap<String, ProjectStateRecord>) -> Self {
        let entries_by_project_key = entries
            .into_iter()
            .map(|entry| (entry.project_key.clone(), entry))
            .collect::<BTreeMap<_, _>>();
        let mut state = Self {
            projects,
            entries_by_project_key,
            filtered_project_keys: Vec::new(),
            selected_project_key: None,
            query: String::new(),
        };
        state.refresh_filtered_projects();
        state
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn selected_index(&self) -> Option<usize> {
        let selected_project_key = self.selected_project_key.as_ref()?;
        self.filtered_project_keys
            .iter()
            .position(|project_key| project_key == selected_project_key)
    }

    pub fn filtered_entries(&self) -> Vec<&PickerEntry> {
        self.filtered_project_keys
            .iter()
            .filter_map(|project_key| self.entries_by_project_key.get(project_key))
            .collect()
    }

    pub fn move_up(&mut self) {
        let Some(selected_index) = self.selected_index() else {
            return;
        };

        if selected_index > 0 {
            self.selected_project_key =
                Some(self.filtered_project_keys[selected_index - 1].clone());
        }
    }

    pub fn move_down(&mut self) {
        let Some(selected_index) = self.selected_index() else {
            return;
        };

        if selected_index + 1 < self.filtered_project_keys.len() {
            self.selected_project_key =
                Some(self.filtered_project_keys[selected_index + 1].clone());
        }
    }

    pub fn confirm(&self) -> PickerOutcome {
        match self.selected_entry() {
            Some(entry) => PickerOutcome::Confirm(entry.clone()),
            None => PickerOutcome::Cancel,
        }
    }

    pub fn cancel(&self) -> PickerOutcome {
        PickerOutcome::Cancel
    }

    pub fn append_query_char(&mut self, character: char) {
        self.query.push(character);
        self.refresh_filtered_projects();
    }

    pub fn pop_query_char(&mut self) {
        if self.query.pop().is_some() {
            self.refresh_filtered_projects();
        }
    }

    pub fn apply_refresh(
        &mut self,
        entries: Vec<PickerEntry>,
        projects: BTreeMap<String, ProjectStateRecord>,
    ) {
        self.projects = projects;
        self.entries_by_project_key = entries
            .into_iter()
            .map(|entry| (entry.project_key.clone(), entry))
            .collect();
        self.refresh_filtered_projects();
    }

    fn selected_entry(&self) -> Option<&PickerEntry> {
        let selected_project_key = self.selected_project_key.as_ref()?;
        self.entries_by_project_key.get(selected_project_key)
    }

    fn refresh_filtered_projects(&mut self) {
        let previous_selection = self.selected_project_key.clone();
        self.filtered_project_keys = rank_project_keys(&self.query, &self.projects)
            .into_iter()
            .filter(|project_key| self.entries_by_project_key.contains_key(project_key))
            .collect();

        self.selected_project_key = match previous_selection {
            Some(previous_selection)
                if self
                    .filtered_project_keys
                    .iter()
                    .any(|project_key| project_key == &previous_selection) =>
            {
                Some(previous_selection)
            }
            _ => self.filtered_project_keys.first().cloned(),
        };
    }
}

pub fn run_picker(
    entries: Vec<PickerEntry>,
    projects: BTreeMap<String, ProjectStateRecord>,
    mut refresh_receiver: Option<Receiver<RefreshMessage>>,
    mut apply_refresh: impl FnMut(
        crate::domain::WindowInventory,
    ) -> Result<PickerRefresh, Report<AppError>>,
    command_span: &tracing::Span,
    run_span: &tracing::Span,
) -> Result<PickerOutcome, Report<AppError>> {
    if entries.is_empty() {
        return Err(Report::new(AppError::Tui).attach("Cannot open picker with no entries"));
    }

    let mut terminal = {
        let _command_enter = command_span.enter();
        let _run_enter = run_span.enter();
        let setup_span = info_span!("tui.setup", entries = entries.len());
        let _setup_enter = setup_span.enter();

        enable_raw_mode()
            .change_context(AppError::Tui)
            .attach("Failed to enable terminal raw mode for switch picker")?;

        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, cursor::Hide)
            .change_context(AppError::Tui)
            .attach("Failed to enter alternate screen for switch picker")?;

        let backend = CrosstermBackend::new(stdout);
        Terminal::new(backend)
            .change_context(AppError::Tui)
            .attach("Failed to initialize terminal backend for switch picker")?
    };
    let _cleanup = TerminalCleanup;

    let mut state = PickerState::new(entries, projects);

    loop {
        if let Some(receiver) = refresh_receiver.as_ref() {
            match receiver.try_recv() {
                Ok(RefreshMessage::Success(inventory)) => {
                    let refresh = apply_refresh(inventory)?;
                    state.apply_refresh(refresh.entries, refresh.projects);
                    refresh_receiver = None;
                }
                Ok(RefreshMessage::Failure(_error)) => {
                    refresh_receiver = None;
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    refresh_receiver = None;
                }
            }
        }

        {
            let _command_enter = command_span.enter();
            let _run_enter = run_span.enter();
            let draw_span = info_span!(
                "tui.draw",
                selected_index = state.selected_index(),
                query = state.query()
            );
            let _draw_enter = draw_span.enter();
            terminal
                .draw(|frame| render_picker(frame, &state))
                .change_context(AppError::Tui)
                .attach("Failed to draw switch picker")?;
        }

        let Some(key_event) = poll_key_event(Duration::from_millis(50))? else {
            continue;
        };

        if let Some(outcome) = {
            let _command_enter = command_span.enter();
            let _run_enter = run_span.enter();
            let handle_span = info_span!("tui.handle_key_event", key = ?key_event.code);
            let _handle_enter = handle_span.enter();
            handle_key_event(key_event, &mut state)
        } {
            return Ok(outcome);
        }
    }
}

struct TerminalCleanup;

impl Drop for TerminalCleanup {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen, cursor::Show);
    }
}

fn poll_key_event(timeout: Duration) -> Result<Option<KeyEvent>, Report<AppError>> {
    let has_event = event::poll(timeout)
        .change_context(AppError::Tui)
        .attach("Failed to poll terminal events for switch picker")?;
    if !has_event {
        return Ok(None);
    }

    let event = event::read()
        .change_context(AppError::Tui)
        .attach("Failed to read terminal events for switch picker")?;

    if let Event::Key(key_event) = event {
        if matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return Ok(Some(key_event));
        }
    }

    Ok(None)
}

fn handle_key_event(key_event: KeyEvent, state: &mut PickerState) -> Option<PickerOutcome> {
    match key_event.code {
        KeyCode::Up | KeyCode::Char('k') => {
            state.move_up();
            None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.move_down();
            None
        }
        KeyCode::Backspace => {
            state.pop_query_char();
            None
        }
        KeyCode::Enter => Some(state.confirm()),
        KeyCode::Esc | KeyCode::Char('q') => Some(state.cancel()),
        KeyCode::Char(character) if is_printable_query_char(character, key_event.modifiers) => {
            state.append_query_char(character);
            None
        }
        _ => None,
    }
}

fn render_picker(frame: &mut Frame, state: &PickerState) {
    let [instructions_area, query_area, list_area] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(3),
        Constraint::Min(1),
    ])
    .areas(frame.area());

    let instructions = Paragraph::new("Up/k Down/j move  Enter select  Esc/q cancel").block(
        Block::default()
            .borders(Borders::ALL)
            .title("Ghostty Switch"),
    );
    frame.render_widget(instructions, instructions_area);

    let query = if state.query().is_empty() {
        "Type to filter projects".to_owned()
    } else {
        state.query().to_owned()
    };
    let query_widget = Paragraph::new(query).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Query (Backspace deletes)"),
    );
    frame.render_widget(query_widget, query_area);

    let filtered_entries = state.filtered_entries();
    if filtered_entries.is_empty() {
        let empty_state = Paragraph::new("No projects match the current query")
            .block(Block::default().borders(Borders::ALL).title("Windows"));
        frame.render_widget(empty_state, list_area);
        return;
    }

    let items: Vec<ListItem> = filtered_entries
        .iter()
        .map(|entry| {
            ListItem::new(vec![
                Line::styled(
                    entry.title.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Line::raw(entry.detail.clone()),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Windows"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">> ");

    let mut list_state = ListState::default().with_selected(state.selected_index());
    frame.render_stateful_widget(list, list_area, &mut list_state);
}

fn is_printable_query_char(character: char, modifiers: KeyModifiers) -> bool {
    (character.is_ascii_graphic() || character == ' ')
        && !modifiers.contains(KeyModifiers::CONTROL)
        && !modifiers.contains(KeyModifiers::ALT)
        && !modifiers.contains(KeyModifiers::SUPER)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use jiff::Timestamp;

    use super::{PickerEntry, PickerOutcome, PickerState, handle_key_event};
    use crate::state::ProjectStateRecord;

    #[test]
    fn first_row_is_selected_initially() {
        let state = PickerState::new(sample_entries(), sample_projects());

        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn moving_up_at_start_stays_on_first_row() {
        let mut state = PickerState::new(sample_entries(), sample_projects());

        state.move_up();

        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn moving_down_at_end_stays_on_last_row() {
        let mut state = PickerState::new(sample_entries(), sample_projects());
        state.move_down();
        state.move_down();
        state.move_down();

        assert_eq!(state.selected_index(), Some(2));
    }

    #[test]
    fn confirm_returns_selected_row() {
        let mut state = PickerState::new(sample_entries(), sample_projects());
        state.move_down();
        state.move_down();

        assert_eq!(
            state.confirm(),
            PickerOutcome::Confirm(PickerEntry {
                project_key: "/tmp/project-b".to_owned(),
                window_id: "window-2".to_owned(),
                title: "project-b".to_owned(),
                detail: "/tmp/project-b | window-2".to_owned(),
            })
        );
    }

    #[test]
    fn cancel_returns_cancel_outcome() {
        let state = PickerState::new(sample_entries(), sample_projects());

        assert_eq!(state.cancel(), PickerOutcome::Cancel);
    }

    #[test]
    fn empty_query_uses_mru_ordering() {
        let state = PickerState::new(sample_entries(), sample_projects());

        assert_eq!(
            state
                .filtered_entries()
                .into_iter()
                .map(|entry| entry.project_key.as_str())
                .collect::<Vec<_>>(),
            vec!["/tmp/project-c", "/tmp/project-a", "/tmp/project-b"]
        );
    }

    #[test]
    fn typing_query_updates_filtered_entries() {
        let mut state = PickerState::new(sample_entries(), sample_projects());

        state.append_query_char('b');

        assert_eq!(state.query(), "b");
        assert_eq!(
            state
                .filtered_entries()
                .into_iter()
                .map(|entry| entry.project_key.as_str())
                .collect::<Vec<_>>(),
            vec!["/tmp/project-b"]
        );
    }

    #[test]
    fn backspace_widens_filtered_entries() {
        let mut state = PickerState::new(sample_entries(), sample_projects());
        state.append_query_char('p');
        state.append_query_char('r');
        state.append_query_char('o');
        state.append_query_char('j');
        state.append_query_char('e');
        state.append_query_char('c');
        state.append_query_char('t');
        state.append_query_char('-');
        state.append_query_char('b');

        state.pop_query_char();

        assert_eq!(state.query(), "project-");
        assert_eq!(state.filtered_entries().len(), 3);
    }

    #[test]
    fn selection_is_preserved_by_project_key_when_still_visible() {
        let mut state = PickerState::new(sample_entries(), sample_projects());
        state.move_down();

        state.append_query_char('a');

        assert_eq!(
            state.confirm(),
            PickerOutcome::Confirm(PickerEntry {
                project_key: "/tmp/project-a".to_owned(),
                window_id: "window-1".to_owned(),
                title: "project-a".to_owned(),
                detail: "/tmp/project-a | window-1".to_owned(),
            })
        );
    }

    #[test]
    fn selection_falls_back_to_first_filtered_result_when_previous_selection_disappears() {
        let mut state = PickerState::new(sample_entries(), sample_projects());
        state.move_down();

        for character in "project-c".chars() {
            state.append_query_char(character);
        }

        assert_eq!(
            state.confirm(),
            PickerOutcome::Confirm(PickerEntry {
                project_key: "/tmp/project-c".to_owned(),
                window_id: "window-3".to_owned(),
                title: "project-c".to_owned(),
                detail: "/tmp/project-c | window-3".to_owned(),
            })
        );
    }

    #[test]
    fn confirm_returns_cancel_when_query_has_no_results() {
        let mut state = PickerState::new(sample_entries(), sample_projects());

        state.append_query_char('z');
        state.append_query_char('z');
        state.append_query_char('z');

        assert_eq!(state.selected_index(), None);
        assert_eq!(state.confirm(), PickerOutcome::Cancel);
    }

    #[test]
    fn handle_key_event_appends_and_deletes_query_text() {
        let mut state = PickerState::new(sample_entries(), sample_projects());

        assert_eq!(
            handle_key_event(key_event(KeyCode::Char('b')), &mut state),
            None
        );
        assert_eq!(
            handle_key_event(key_event(KeyCode::Backspace), &mut state),
            None
        );

        assert_eq!(state.query(), "");
    }

    #[test]
    fn apply_refresh_preserves_query() {
        let projects = sample_projects();
        let mut state = PickerState::new(sample_entries(), projects.clone());
        for character in "project-c".chars() {
            state.append_query_char(character);
        }

        state.apply_refresh(sample_entries(), projects);

        assert_eq!(state.query(), "project-c");
        assert_eq!(
            state
                .filtered_entries()
                .into_iter()
                .map(|entry| entry.project_key.as_str())
                .collect::<Vec<_>>(),
            vec!["/tmp/project-c"]
        );
    }

    #[test]
    fn apply_refresh_preserves_selection_when_selected_project_still_exists() {
        let projects = sample_projects();
        let mut state = PickerState::new(sample_entries(), projects.clone());
        state.move_down();

        state.apply_refresh(sample_entries(), projects);

        assert_eq!(
            state.confirm(),
            PickerOutcome::Confirm(PickerEntry {
                project_key: "/tmp/project-a".to_owned(),
                window_id: "window-1".to_owned(),
                title: "project-a".to_owned(),
                detail: "/tmp/project-a | window-1".to_owned(),
            })
        );
    }

    #[test]
    fn apply_refresh_falls_back_to_first_filtered_result_when_selected_project_disappears() {
        let projects = sample_projects();
        let mut state = PickerState::new(sample_entries(), projects.clone());
        state.move_down();

        state.apply_refresh(
            vec![
                PickerEntry {
                    project_key: "/tmp/project-b".to_owned(),
                    window_id: "window-2".to_owned(),
                    title: "project-b".to_owned(),
                    detail: "/tmp/project-b | window-2".to_owned(),
                },
                PickerEntry {
                    project_key: "/tmp/project-c".to_owned(),
                    window_id: "window-3".to_owned(),
                    title: "project-c".to_owned(),
                    detail: "/tmp/project-c | window-3".to_owned(),
                },
            ],
            projects,
        );

        assert_eq!(
            state.confirm(),
            PickerOutcome::Confirm(PickerEntry {
                project_key: "/tmp/project-c".to_owned(),
                window_id: "window-3".to_owned(),
                title: "project-c".to_owned(),
                detail: "/tmp/project-c | window-3".to_owned(),
            })
        );
    }

    #[test]
    fn apply_refresh_can_add_new_projects_without_breaking_filtered_ordering() {
        let mut projects = sample_projects();
        let mut state = PickerState::new(sample_entries(), projects.clone());

        projects.insert(
            "/tmp/project-d".to_owned(),
            project_record("2026-04-16T12:00:00Z"),
        );
        state.apply_refresh(
            vec![
                PickerEntry {
                    project_key: "/tmp/project-a".to_owned(),
                    window_id: "window-1".to_owned(),
                    title: "project-a".to_owned(),
                    detail: "/tmp/project-a | window-1".to_owned(),
                },
                PickerEntry {
                    project_key: "/tmp/project-b".to_owned(),
                    window_id: "window-2".to_owned(),
                    title: "project-b".to_owned(),
                    detail: "/tmp/project-b | window-2".to_owned(),
                },
                PickerEntry {
                    project_key: "/tmp/project-c".to_owned(),
                    window_id: "window-3".to_owned(),
                    title: "project-c".to_owned(),
                    detail: "/tmp/project-c | window-3".to_owned(),
                },
                PickerEntry {
                    project_key: "/tmp/project-d".to_owned(),
                    window_id: "window-4".to_owned(),
                    title: "project-d".to_owned(),
                    detail: "/tmp/project-d | window-4".to_owned(),
                },
            ],
            projects,
        );

        assert_eq!(
            state
                .filtered_entries()
                .into_iter()
                .map(|entry| entry.project_key.as_str())
                .collect::<Vec<_>>(),
            vec![
                "/tmp/project-d",
                "/tmp/project-c",
                "/tmp/project-a",
                "/tmp/project-b",
            ]
        );
    }

    #[test]
    fn apply_refresh_keeps_empty_state_behavior() {
        let projects = sample_projects();
        let mut state = PickerState::new(sample_entries(), projects.clone());
        state.append_query_char('z');

        state.apply_refresh(sample_entries(), projects);

        assert_eq!(state.selected_index(), None);
        assert_eq!(state.confirm(), PickerOutcome::Cancel);
    }

    #[test]
    fn apply_refresh_searches_against_new_projects() {
        let mut projects = sample_projects();
        let mut state = PickerState::new(sample_entries(), projects.clone());

        projects.insert(
            "/tmp/project-d".to_owned(),
            project_record("2026-04-16T12:00:00Z"),
        );
        state.apply_refresh(refreshed_entries(), projects);

        state.append_query_char('d');

        assert_eq!(
            state.confirm(),
            PickerOutcome::Confirm(PickerEntry {
                project_key: "/tmp/project-d".to_owned(),
                window_id: "window-4".to_owned(),
                title: "project-d".to_owned(),
                detail: "/tmp/project-d | window-4".to_owned(),
            })
        );
    }

    #[test]
    fn apply_refresh_keeps_new_projects_visible_after_clearing_query() {
        let mut projects = sample_projects();
        let mut state = PickerState::new(sample_entries(), projects.clone());

        projects.insert(
            "/tmp/project-d".to_owned(),
            project_record("2026-04-16T12:00:00Z"),
        );
        state.apply_refresh(refreshed_entries(), projects);

        state.append_query_char('d');
        state.pop_query_char();

        assert_eq!(
            state
                .filtered_entries()
                .into_iter()
                .map(|entry| entry.project_key.as_str())
                .collect::<Vec<_>>(),
            vec![
                "/tmp/project-d",
                "/tmp/project-c",
                "/tmp/project-a",
                "/tmp/project-b",
            ]
        );
    }

    fn sample_entries() -> Vec<PickerEntry> {
        vec![
            PickerEntry {
                project_key: "/tmp/project-a".to_owned(),
                window_id: "window-1".to_owned(),
                title: "project-a".to_owned(),
                detail: "/tmp/project-a | window-1".to_owned(),
            },
            PickerEntry {
                project_key: "/tmp/project-b".to_owned(),
                window_id: "window-2".to_owned(),
                title: "project-b".to_owned(),
                detail: "/tmp/project-b | window-2".to_owned(),
            },
            PickerEntry {
                project_key: "/tmp/project-c".to_owned(),
                window_id: "window-3".to_owned(),
                title: "project-c".to_owned(),
                detail: "/tmp/project-c | window-3".to_owned(),
            },
        ]
    }

    fn refreshed_entries() -> Vec<PickerEntry> {
        vec![
            PickerEntry {
                project_key: "/tmp/project-a".to_owned(),
                window_id: "window-1".to_owned(),
                title: "project-a".to_owned(),
                detail: "/tmp/project-a | window-1".to_owned(),
            },
            PickerEntry {
                project_key: "/tmp/project-b".to_owned(),
                window_id: "window-2".to_owned(),
                title: "project-b".to_owned(),
                detail: "/tmp/project-b | window-2".to_owned(),
            },
            PickerEntry {
                project_key: "/tmp/project-c".to_owned(),
                window_id: "window-3".to_owned(),
                title: "project-c".to_owned(),
                detail: "/tmp/project-c | window-3".to_owned(),
            },
            PickerEntry {
                project_key: "/tmp/project-d".to_owned(),
                window_id: "window-4".to_owned(),
                title: "project-d".to_owned(),
                detail: "/tmp/project-d | window-4".to_owned(),
            },
        ]
    }

    fn sample_projects() -> BTreeMap<String, ProjectStateRecord> {
        BTreeMap::from([
            (
                "/tmp/project-a".to_owned(),
                project_record("2026-04-16T10:00:00Z"),
            ),
            (
                "/tmp/project-b".to_owned(),
                project_record("2026-04-16T09:00:00Z"),
            ),
            (
                "/tmp/project-c".to_owned(),
                project_record("2026-04-16T11:00:00Z"),
            ),
        ])
    }

    fn project_record(last_accessed_at: &str) -> ProjectStateRecord {
        ProjectStateRecord {
            last_accessed_at: parse_timestamp(last_accessed_at),
            last_seen_at: parse_timestamp("2026-04-16T12:00:00Z"),
            last_window_id: "window".to_owned(),
            last_window_name: Some("Workspace".to_owned()),
        }
    }

    fn key_event(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn parse_timestamp(input: &str) -> Timestamp {
        input.parse().expect("timestamp fixture should parse")
    }
}
