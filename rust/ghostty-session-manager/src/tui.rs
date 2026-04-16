use std::io;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
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

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerEntry {
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
pub struct PickerState {
    entries: Vec<PickerEntry>,
    selected_index: usize,
}

impl PickerState {
    pub fn new(entries: Vec<PickerEntry>) -> Self {
        Self {
            entries,
            selected_index: 0,
        }
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected_index + 1 < self.entries.len() {
            self.selected_index += 1;
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

    fn selected_entry(&self) -> Option<&PickerEntry> {
        self.entries.get(self.selected_index)
    }
}

pub fn run_picker(entries: Vec<PickerEntry>) -> Result<PickerOutcome, Report<AppError>> {
    if entries.is_empty() {
        return Err(Report::new(AppError::Tui).attach("Cannot open picker with no entries"));
    }

    enable_raw_mode()
        .change_context(AppError::Tui)
        .attach("Failed to enable terminal raw mode for switch picker")?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide)
        .change_context(AppError::Tui)
        .attach("Failed to enter alternate screen for switch picker")?;
    let _cleanup = TerminalCleanup;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)
        .change_context(AppError::Tui)
        .attach("Failed to initialize terminal backend for switch picker")?;

    let mut state = PickerState::new(entries);

    loop {
        terminal
            .draw(|frame| render_picker(frame, &state))
            .change_context(AppError::Tui)
            .attach("Failed to draw switch picker")?;

        if let Some(outcome) = handle_key_event(read_key_event()?, &mut state) {
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

fn read_key_event() -> Result<KeyEvent, Report<AppError>> {
    loop {
        let event = event::read()
            .change_context(AppError::Tui)
            .attach("Failed to read terminal events for switch picker")?;

        if let Event::Key(key_event) = event {
            if matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                return Ok(key_event);
            }
        }
    }
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
        KeyCode::Enter => Some(state.confirm()),
        KeyCode::Esc | KeyCode::Char('q') => Some(state.cancel()),
        _ => None,
    }
}

fn render_picker(frame: &mut Frame, state: &PickerState) {
    let [instructions_area, list_area] =
        Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).areas(frame.area());

    let instructions = Paragraph::new("Up/k Down/j move  Enter select  Esc/q cancel").block(
        Block::default()
            .borders(Borders::ALL)
            .title("Ghostty Switch"),
    );
    frame.render_widget(instructions, instructions_area);

    let items: Vec<ListItem> = state
        .entries
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

    let mut list_state = ListState::default().with_selected(Some(state.selected_index));
    frame.render_stateful_widget(list, list_area, &mut list_state);
}

#[cfg(test)]
mod tests {
    use super::{PickerEntry, PickerOutcome, PickerState};

    #[test]
    fn first_row_is_selected_initially() {
        let state = PickerState::new(sample_entries());

        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn moving_up_at_start_stays_on_first_row() {
        let mut state = PickerState::new(sample_entries());

        state.move_up();

        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn moving_down_at_end_stays_on_last_row() {
        let mut state = PickerState::new(sample_entries());
        state.move_down();
        state.move_down();
        state.move_down();

        assert_eq!(state.selected_index(), 1);
    }

    #[test]
    fn confirm_returns_selected_row() {
        let mut state = PickerState::new(sample_entries());
        state.move_down();

        assert_eq!(
            state.confirm(),
            PickerOutcome::Confirm(PickerEntry {
                window_id: "window-2".to_owned(),
                title: "project-b".to_owned(),
                detail: "/tmp/project-b | window-2".to_owned(),
            })
        );
    }

    #[test]
    fn cancel_returns_cancel_outcome() {
        let state = PickerState::new(sample_entries());

        assert_eq!(state.cancel(), PickerOutcome::Cancel);
    }

    fn sample_entries() -> Vec<PickerEntry> {
        vec![
            PickerEntry {
                window_id: "window-1".to_owned(),
                title: "project-a".to_owned(),
                detail: "/tmp/project-a | window-1".to_owned(),
            },
            PickerEntry {
                window_id: "window-2".to_owned(),
                title: "project-b".to_owned(),
                detail: "/tmp/project-b | window-2".to_owned(),
            },
        ]
    }
}
