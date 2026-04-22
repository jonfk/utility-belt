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
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{HighlightSpacing, List, ListItem, ListState, Paragraph},
};
use tracing::info_span;

use crate::error::AppError;
use crate::search::{ProjectMatch, ProjectMatchField, RankedProjectMatch, rank_projects};
use crate::state::ProjectStateRecord;

const NARROW_ROW_WIDTH: u16 = 72;
const PROMPT_PLACEHOLDER: &str = "Type to filter projects";
const FOOTER_HELP: &str = "Enter select  Esc cancel  Ctrl-N/P move  Ctrl-U clear  Ctrl-W delete";
const SELECTED_ROW_BG: Color = Color::Rgb(26, 47, 67);
const SELECTED_ROW_FG: Color = Color::White;
const PRIMARY_TEXT: Color = Color::Gray;
const MUTED_TEXT: Color = Color::DarkGray;
const MATCH_TEXT: Color = Color::Cyan;
const ERROR_TEXT: Color = Color::LightRed;
const STATUS_TEXT: Color = Color::LightCyan;

#[derive(Debug)]
pub enum RefreshMessage {
    Success(crate::domain::WindowInventory),
    Failure(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerEntry {
    pub project_key: String,
    pub window_id: String,
    pub primary_label: String,
    pub secondary_path: Option<String>,
    pub window_name: Option<String>,
}

impl PickerEntry {
    fn status_label(&self) -> String {
        match self.window_name.as_deref() {
            Some(window_name) => format!("{window_name} | {}", self.window_id),
            None => self.window_id.clone(),
        }
    }
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
enum RefreshState {
    Idle,
    Refreshing,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerState {
    projects: BTreeMap<String, ProjectStateRecord>,
    entries_by_project_key: BTreeMap<String, PickerEntry>,
    filtered_projects: Vec<RankedProjectMatch>,
    selected_project_key: Option<String>,
    current_project_key: Option<String>,
    query: String,
    refresh_state: RefreshState,
}

impl PickerState {
    pub fn new(
        entries: Vec<PickerEntry>,
        projects: BTreeMap<String, ProjectStateRecord>,
        current_project_key: Option<String>,
    ) -> Self {
        let entries_by_project_key = entries
            .into_iter()
            .map(|entry| (entry.project_key.clone(), entry))
            .collect::<BTreeMap<_, _>>();
        let mut state = Self {
            projects,
            entries_by_project_key,
            filtered_projects: Vec::new(),
            selected_project_key: None,
            current_project_key,
            query: String::new(),
            refresh_state: RefreshState::Idle,
        };
        state.refresh_filtered_projects();
        state
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn selected_index(&self) -> Option<usize> {
        let selected_project_key = self.selected_project_key.as_ref()?;
        self.filtered_projects
            .iter()
            .position(|project_match| &project_match.project_key == selected_project_key)
    }

    fn filtered_render_entries(&self) -> Vec<(&PickerEntry, &RankedProjectMatch)> {
        self.filtered_projects
            .iter()
            .filter_map(|project_match| {
                self.entries_by_project_key
                    .get(&project_match.project_key)
                    .map(|entry| (entry, project_match))
            })
            .collect()
    }

    pub fn filtered_count(&self) -> usize {
        self.filtered_projects.len()
    }

    pub fn total_count(&self) -> usize {
        self.entries_by_project_key.len()
    }

    pub fn move_up(&mut self) {
        let Some(selected_index) = self.selected_index() else {
            return;
        };

        if selected_index > 0 {
            self.selected_project_key = Some(
                self.filtered_projects[selected_index - 1]
                    .project_key
                    .clone(),
            );
        }
    }

    pub fn move_down(&mut self) {
        let Some(selected_index) = self.selected_index() else {
            return;
        };

        if selected_index + 1 < self.filtered_projects.len() {
            self.selected_project_key = Some(
                self.filtered_projects[selected_index + 1]
                    .project_key
                    .clone(),
            );
        }
    }

    pub fn move_to_first(&mut self) {
        self.selected_project_key = self
            .filtered_projects
            .first()
            .map(|project_match| project_match.project_key.clone());
    }

    pub fn move_to_last(&mut self) {
        self.selected_project_key = self
            .filtered_projects
            .last()
            .map(|project_match| project_match.project_key.clone());
    }

    pub fn move_page_up(&mut self, page_size: usize) {
        let Some(selected_index) = self.selected_index() else {
            return;
        };

        let next_index = selected_index.saturating_sub(page_size.max(1));
        self.selected_project_key = Some(self.filtered_projects[next_index].project_key.clone());
    }

    pub fn move_page_down(&mut self, page_size: usize) {
        let Some(selected_index) = self.selected_index() else {
            return;
        };

        let next_index = (selected_index + page_size.max(1)).min(self.filtered_projects.len() - 1);
        self.selected_project_key = Some(self.filtered_projects[next_index].project_key.clone());
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

    pub fn pop_query_word(&mut self) {
        let original_query = self.query.clone();

        while self.query.ends_with(' ') {
            self.query.pop();
        }

        while self
            .query
            .chars()
            .last()
            .is_some_and(|character| !character.is_whitespace())
        {
            self.query.pop();
        }

        if self.query != original_query {
            self.refresh_filtered_projects();
        }
    }

    pub fn clear_query(&mut self) {
        if !self.query.is_empty() {
            self.query.clear();
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

    pub fn set_refreshing(&mut self) {
        self.refresh_state = RefreshState::Refreshing;
    }

    pub fn set_refresh_idle(&mut self) {
        self.refresh_state = RefreshState::Idle;
    }

    pub fn set_refresh_failed(&mut self, error: String) {
        self.refresh_state = RefreshState::Failed(error);
    }

    fn selected_entry(&self) -> Option<&PickerEntry> {
        let selected_project_key = self.selected_project_key.as_ref()?;
        self.entries_by_project_key.get(selected_project_key)
    }

    fn refresh_filtered_projects(&mut self) {
        self.filtered_projects = rank_projects(
            &self.query,
            &self.projects,
            self.current_project_key.as_deref(),
        )
        .into_iter()
        .filter(|project_match| {
            self.entries_by_project_key
                .contains_key(&project_match.project_key)
        })
        .collect();

        self.selected_project_key = self
            .filtered_projects
            .first()
            .map(|project_match| project_match.project_key.clone());
    }
}

pub fn run_picker(
    entries: Vec<PickerEntry>,
    projects: BTreeMap<String, ProjectStateRecord>,
    current_project_key: Option<String>,
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

    let mut state = PickerState::new(entries, projects, current_project_key);
    if refresh_receiver.is_some() {
        state.set_refreshing();
    }

    loop {
        if let Some(receiver) = refresh_receiver.as_ref() {
            match receiver.try_recv() {
                Ok(RefreshMessage::Success(inventory)) => {
                    let refresh = apply_refresh(inventory)?;
                    state.apply_refresh(refresh.entries, refresh.projects);
                    state.set_refresh_idle();
                    refresh_receiver = None;
                }
                Ok(RefreshMessage::Failure(error)) => {
                    state.set_refresh_failed(error);
                    refresh_receiver = None;
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    state.set_refresh_idle();
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

        let page_size = terminal
            .size()
            .change_context(AppError::Tui)
            .attach("Failed to read terminal size for switch picker")
            .map(|size| page_size_for_area(Rect::new(0, 0, size.width, size.height)))?;

        if let Some(outcome) = {
            let _command_enter = command_span.enter();
            let _run_enter = run_span.enter();
            let handle_span = info_span!("tui.handle_key_event", key = ?key_event.code);
            let _handle_enter = handle_span.enter();
            handle_key_event(key_event, &mut state, page_size)
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

fn handle_key_event(
    key_event: KeyEvent,
    state: &mut PickerState,
    page_size: usize,
) -> Option<PickerOutcome> {
    match key_event.code {
        KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(state.cancel())
        }
        KeyCode::Up => {
            state.move_up();
            None
        }
        KeyCode::Down => {
            state.move_down();
            None
        }
        KeyCode::Char('k') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            state.move_up();
            None
        }
        KeyCode::Char('j') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            state.move_down();
            None
        }
        KeyCode::Char('p') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            state.move_up();
            None
        }
        KeyCode::Char('n') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            state.move_down();
            None
        }
        KeyCode::Home => {
            state.move_to_first();
            None
        }
        KeyCode::End => {
            state.move_to_last();
            None
        }
        KeyCode::PageUp => {
            state.move_page_up(page_size);
            None
        }
        KeyCode::PageDown => {
            state.move_page_down(page_size);
            None
        }
        KeyCode::Backspace if key_event.modifiers.contains(KeyModifiers::ALT) => {
            state.pop_query_word();
            None
        }
        KeyCode::Char('u') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            state.clear_query();
            None
        }
        KeyCode::Char('w') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            state.pop_query_word();
            None
        }
        KeyCode::Backspace => {
            state.pop_query_char();
            None
        }
        KeyCode::Enter => Some(state.confirm()),
        KeyCode::Esc => Some(state.cancel()),
        KeyCode::Char(character) if is_printable_query_char(character, key_event.modifiers) => {
            state.append_query_char(character);
            None
        }
        _ => None,
    }
}

fn render_picker(frame: &mut Frame, state: &PickerState) {
    let [prompt_area, list_area, footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    render_prompt(frame, prompt_area, state);
    render_results(frame, list_area, state);
    render_footer(frame, footer_area, state);
}

fn render_prompt(frame: &mut Frame, area: Rect, state: &PickerState) {
    let count = format!("{}/{}", state.filtered_count(), state.total_count());
    let [prompt_text_area, count_area] = Layout::horizontal([
        Constraint::Min(1),
        Constraint::Length(count.chars().count() as u16),
    ])
    .areas(area);

    let prompt_line = if state.query().is_empty() {
        Line::from(vec![
            Span::styled("> ", Style::default().fg(STATUS_TEXT)),
            Span::styled(PROMPT_PLACEHOLDER, Style::default().fg(MUTED_TEXT)),
        ])
    } else {
        Line::from(vec![
            Span::styled("> ", Style::default().fg(STATUS_TEXT)),
            Span::styled(
                state.query().to_owned(),
                Style::default()
                    .fg(PRIMARY_TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    };

    frame.render_widget(Paragraph::new(prompt_line), prompt_text_area);
    frame.render_widget(
        Paragraph::new(count).style(Style::default().fg(MUTED_TEXT)),
        count_area,
    );
}

fn render_results(frame: &mut Frame, area: Rect, state: &PickerState) {
    let filtered_entries = state.filtered_render_entries();
    if filtered_entries.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                "No matches",
                Style::default().fg(MUTED_TEXT),
            )])),
            area,
        );
        return;
    }

    let use_narrow_layout = area.width < NARROW_ROW_WIDTH;
    let items = filtered_entries
        .iter()
        .map(|(entry, project_match)| {
            build_list_item(
                entry,
                project_match.project_match.as_ref(),
                use_narrow_layout,
            )
        })
        .collect::<Vec<_>>();

    let list = List::new(items)
        .highlight_symbol("> ")
        .highlight_spacing(HighlightSpacing::Always)
        .highlight_style(Style::default().bg(SELECTED_ROW_BG).fg(SELECTED_ROW_FG));

    let mut list_state = ListState::default().with_selected(state.selected_index());
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_footer(frame: &mut Frame, area: Rect, state: &PickerState) {
    let help = Paragraph::new(FOOTER_HELP).style(Style::default().fg(MUTED_TEXT));
    let Some((status_text, status_style)) = footer_status(state) else {
        frame.render_widget(help, area);
        return;
    };

    let status_width = status_text.chars().count() as u16;
    let [help_area, status_area] =
        Layout::horizontal([Constraint::Min(1), Constraint::Length(status_width)]).areas(area);

    frame.render_widget(help, help_area);
    frame.render_widget(Paragraph::new(status_text).style(status_style), status_area);
}

fn footer_status(state: &PickerState) -> Option<(String, Style)> {
    match &state.refresh_state {
        RefreshState::Idle => None,
        RefreshState::Refreshing => {
            Some(("refreshing...".to_owned(), Style::default().fg(STATUS_TEXT)))
        }
        RefreshState::Failed(error) => Some((
            format!("refresh failed: {error}"),
            Style::default().fg(ERROR_TEXT),
        )),
    }
}

fn build_list_item(
    entry: &PickerEntry,
    project_match: Option<&ProjectMatch>,
    use_narrow_layout: bool,
) -> ListItem<'static> {
    let primary_indices = match_indices(project_match, ProjectMatchField::Basename);
    let path_indices = match_indices(project_match, ProjectMatchField::FullPath);
    let primary_spans = highlighted_spans(
        &entry.primary_label,
        primary_indices,
        Style::default()
            .fg(PRIMARY_TEXT)
            .add_modifier(Modifier::BOLD),
        Style::default().fg(MATCH_TEXT).add_modifier(Modifier::BOLD),
    );
    let path_spans = entry.secondary_path.as_deref().map(|secondary_path| {
        highlighted_spans(
            secondary_path,
            path_indices,
            Style::default().fg(MUTED_TEXT),
            Style::default().fg(MATCH_TEXT).add_modifier(Modifier::BOLD),
        )
    });
    let metadata = entry.status_label();

    let lines = if use_narrow_layout {
        let mut primary_line = primary_spans;
        if !metadata.is_empty() {
            primary_line.push(Span::styled("  ", Style::default().fg(MUTED_TEXT)));
            primary_line.push(Span::styled(metadata, Style::default().fg(MUTED_TEXT)));
        }

        let mut lines = vec![Line::from(primary_line)];
        if let Some(path_spans) = path_spans {
            lines.push(Line::from(path_spans));
        }
        lines
    } else {
        let mut line = primary_spans;
        if let Some(path_spans) = path_spans {
            line.push(Span::styled("    ", Style::default().fg(MUTED_TEXT)));
            line.extend(path_spans);
        }
        if !metadata.is_empty() {
            line.push(Span::styled("   ", Style::default().fg(MUTED_TEXT)));
            line.push(Span::styled(metadata, Style::default().fg(MUTED_TEXT)));
        }
        vec![Line::from(line)]
    };

    ListItem::new(lines)
}

fn match_indices(project_match: Option<&ProjectMatch>, field: ProjectMatchField) -> &[usize] {
    match project_match {
        Some(project_match) if project_match.field == field => &project_match.char_indices,
        _ => &[],
    }
}

fn highlighted_spans(
    text: &str,
    highlighted_indices: &[usize],
    base_style: Style,
    highlight_style: Style,
) -> Vec<Span<'static>> {
    if text.is_empty() {
        return vec![Span::styled(String::new(), base_style)];
    }

    if highlighted_indices.is_empty() {
        return vec![Span::styled(text.to_owned(), base_style)];
    }

    let mut spans = Vec::new();
    let mut current_text = String::new();
    let mut current_highlight: Option<bool> = None;

    for (char_index, character) in text.chars().enumerate() {
        let is_highlighted = highlighted_indices.binary_search(&char_index).is_ok();

        match current_highlight {
            Some(previous) if previous != is_highlighted => {
                let style = if previous {
                    base_style.patch(highlight_style)
                } else {
                    base_style
                };
                spans.push(Span::styled(std::mem::take(&mut current_text), style));
                current_text.push(character);
                current_highlight = Some(is_highlighted);
            }
            Some(_) => {
                current_text.push(character);
            }
            None => {
                current_text.push(character);
                current_highlight = Some(is_highlighted);
            }
        }
    }

    if !current_text.is_empty() {
        let style = if current_highlight == Some(true) {
            base_style.patch(highlight_style)
        } else {
            base_style
        };
        spans.push(Span::styled(current_text, style));
    }

    spans
}

fn page_size_for_area(area: Rect) -> usize {
    let [_prompt_area, list_area, _footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(area);
    list_area.height.max(1) as usize
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
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    use super::{
        PickerEntry, PickerOutcome, PickerState, SELECTED_ROW_BG, handle_key_event, render_picker,
    };
    use crate::state::ProjectStateRecord;

    #[test]
    fn first_row_is_selected_initially() {
        let state = PickerState::new(sample_entries(), sample_projects(), None);

        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn moving_up_at_start_stays_on_first_row() {
        let mut state = PickerState::new(sample_entries(), sample_projects(), None);

        state.move_up();

        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn moving_down_at_end_stays_on_last_row() {
        let mut state = PickerState::new(sample_entries(), sample_projects(), None);
        state.move_down();
        state.move_down();
        state.move_down();

        assert_eq!(state.selected_index(), Some(2));
    }

    #[test]
    fn home_and_end_jump_to_list_boundaries() {
        let mut state = PickerState::new(refreshed_entries(), refreshed_projects(), None);
        state.move_down();

        state.move_to_last();
        assert_eq!(state.selected_index(), Some(3));

        state.move_to_first();
        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn page_navigation_uses_visible_page_size() {
        let mut state = PickerState::new(many_entries(), many_projects(), None);

        state.move_page_down(3);
        assert_eq!(state.selected_index(), Some(3));

        state.move_page_up(2);
        assert_eq!(state.selected_index(), Some(1));
    }

    #[test]
    fn confirm_returns_selected_row() {
        let mut state = PickerState::new(sample_entries(), sample_projects(), None);
        state.move_down();
        state.move_down();

        assert_eq!(
            state.confirm(),
            PickerOutcome::Confirm(sample_entry("b", "window-2"))
        );
    }

    #[test]
    fn cancel_returns_cancel_outcome() {
        let state = PickerState::new(sample_entries(), sample_projects(), None);

        assert_eq!(state.cancel(), PickerOutcome::Cancel);
    }

    #[test]
    fn empty_query_uses_mru_ordering() {
        let state = PickerState::new(sample_entries(), sample_projects(), None);

        assert_eq!(
            filtered_project_keys(&state),
            vec!["/tmp/project-c", "/tmp/project-a", "/tmp/project-b"]
        );
    }

    #[test]
    fn empty_query_demotes_current_project_from_initial_order() {
        let state = PickerState::new(
            sample_entries(),
            sample_projects(),
            Some("/tmp/project-c".to_owned()),
        );

        assert_eq!(
            filtered_project_keys(&state),
            vec!["/tmp/project-a", "/tmp/project-b", "/tmp/project-c"]
        );
    }

    #[test]
    fn typing_query_updates_filtered_entries() {
        let mut state = PickerState::new(sample_entries(), sample_projects(), None);

        state.append_query_char('b');

        assert_eq!(state.query(), "b");
        assert_eq!(filtered_project_keys(&state), vec!["/tmp/project-b"]);
    }

    #[test]
    fn typing_query_preserves_current_project_demotion_on_tied_matches() {
        let mut state = PickerState::new(
            sample_entries(),
            sample_projects(),
            Some("/tmp/project-a".to_owned()),
        );

        for character in "project-".chars() {
            state.append_query_char(character);
        }

        assert_eq!(
            filtered_project_keys(&state),
            vec!["/tmp/project-c", "/tmp/project-b", "/tmp/project-a"]
        );
    }

    #[test]
    fn backspace_widens_filtered_entries() {
        let mut state = PickerState::new(sample_entries(), sample_projects(), None);
        for character in "project-b".chars() {
            state.append_query_char(character);
        }

        state.pop_query_char();

        assert_eq!(state.query(), "project-");
        assert_eq!(filtered_project_keys(&state).len(), 3);
        assert_eq!(state.selected_index(), Some(0));
        assert_eq!(
            state.confirm(),
            PickerOutcome::Confirm(sample_entry("c", "window-3"))
        );
    }

    #[test]
    fn ctrl_u_clears_query() {
        let mut state = PickerState::new(sample_entries(), sample_projects(), None);
        for character in "project-b".chars() {
            state.append_query_char(character);
        }

        assert_eq!(handle_key_event(ctrl_key_event('u'), &mut state, 5), None);

        assert_eq!(state.query(), "");
        assert_eq!(state.selected_index(), Some(0));
        assert_eq!(
            state.confirm(),
            PickerOutcome::Confirm(sample_entry("c", "window-3"))
        );
    }

    #[test]
    fn clearing_query_restores_demoted_empty_query_order() {
        let mut state = PickerState::new(
            sample_entries(),
            sample_projects(),
            Some("/tmp/project-c".to_owned()),
        );
        state.append_query_char('b');

        state.clear_query();

        assert_eq!(
            filtered_project_keys(&state),
            vec!["/tmp/project-a", "/tmp/project-b", "/tmp/project-c"]
        );
        assert_eq!(state.selected_index(), Some(0));
        assert_eq!(
            state.confirm(),
            PickerOutcome::Confirm(sample_entry("a", "window-1"))
        );
    }

    #[test]
    fn ctrl_w_deletes_previous_word() {
        let mut state = PickerState::new(sample_entries(), sample_projects(), None);
        for character in "project alpha".chars() {
            state.append_query_char(character);
        }

        assert_eq!(handle_key_event(ctrl_key_event('w'), &mut state, 5), None);

        assert_eq!(state.query(), "project ");
        assert_eq!(state.selected_index(), Some(0));
        assert_eq!(
            state.confirm(),
            PickerOutcome::Confirm(sample_entry("c", "window-3"))
        );
    }

    #[test]
    fn alt_backspace_deletes_previous_word() {
        let mut state = PickerState::new(sample_entries(), sample_projects(), None);
        for character in "project alpha".chars() {
            state.append_query_char(character);
        }

        assert_eq!(
            handle_key_event(alt_backspace_key_event(), &mut state, 5),
            None
        );

        assert_eq!(state.query(), "project ");
        assert_eq!(state.selected_index(), Some(0));
        assert_eq!(
            state.confirm(),
            PickerOutcome::Confirm(sample_entry("c", "window-3"))
        );
    }

    #[test]
    fn typing_query_resets_selection_to_first_filtered_result() {
        let mut state = PickerState::new(sample_entries(), sample_projects(), None);
        state.move_down();

        state.append_query_char('a');

        assert_eq!(state.selected_index(), Some(0));
        assert_eq!(
            state.confirm(),
            PickerOutcome::Confirm(sample_entry("a", "window-1"))
        );
    }

    #[test]
    fn selection_falls_back_to_first_filtered_result_when_previous_selection_disappears() {
        let mut state = PickerState::new(sample_entries(), sample_projects(), None);
        state.move_down();
        state.move_down();

        for character in "project-c".chars() {
            state.append_query_char(character);
        }

        assert_eq!(
            state.confirm(),
            PickerOutcome::Confirm(sample_entry("c", "window-3"))
        );
    }

    #[test]
    fn confirm_returns_cancel_when_query_has_no_results() {
        let mut state = PickerState::new(sample_entries(), sample_projects(), None);

        state.append_query_char('z');
        state.append_query_char('z');
        state.append_query_char('z');

        assert_eq!(state.selected_index(), None);
        assert_eq!(state.confirm(), PickerOutcome::Cancel);
    }

    #[test]
    fn handle_key_event_appends_and_deletes_query_text() {
        let mut state = PickerState::new(sample_entries(), sample_projects(), None);

        assert_eq!(
            handle_key_event(key_event(KeyCode::Char('b')), &mut state, 5),
            None
        );
        assert_eq!(
            handle_key_event(key_event(KeyCode::Backspace), &mut state, 5),
            None
        );

        assert_eq!(state.query(), "");
    }

    #[test]
    fn handle_key_event_ctrl_n_and_ctrl_p_move_selection() {
        let mut state = PickerState::new(sample_entries(), sample_projects(), None);

        assert_eq!(handle_key_event(ctrl_key_event('n'), &mut state, 5), None);
        assert_eq!(state.selected_index(), Some(1));

        assert_eq!(handle_key_event(ctrl_key_event('p'), &mut state, 5), None);
        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn handle_key_event_ctrl_j_and_ctrl_k_move_selection_without_stealing_letters() {
        let mut state = PickerState::new(sample_entries(), sample_projects(), None);

        assert_eq!(handle_key_event(ctrl_key_event('j'), &mut state, 5), None);
        assert_eq!(state.selected_index(), Some(1));

        assert_eq!(handle_key_event(ctrl_key_event('k'), &mut state, 5), None);
        assert_eq!(state.selected_index(), Some(0));

        assert_eq!(
            handle_key_event(key_event(KeyCode::Char('j')), &mut state, 5),
            None
        );
        assert_eq!(state.query(), "j");
    }

    #[test]
    fn handle_key_event_ctrl_c_returns_cancel() {
        let mut state = PickerState::new(sample_entries(), sample_projects(), None);

        assert_eq!(
            handle_key_event(ctrl_key_event('c'), &mut state, 5),
            Some(PickerOutcome::Cancel)
        );
    }

    #[test]
    fn apply_refresh_preserves_query() {
        let projects = sample_projects();
        let mut state = PickerState::new(sample_entries(), projects.clone(), None);
        for character in "project-c".chars() {
            state.append_query_char(character);
        }

        state.apply_refresh(sample_entries(), projects);

        assert_eq!(state.query(), "project-c");
        assert_eq!(filtered_project_keys(&state), vec!["/tmp/project-c"]);
    }

    #[test]
    fn apply_refresh_keeps_using_current_project_key_for_ordering() {
        let mut projects = sample_projects();
        let mut state = PickerState::new(
            sample_entries(),
            projects.clone(),
            Some("/tmp/project-c".to_owned()),
        );

        projects.insert(
            "/tmp/project-d".to_owned(),
            project_record("2026-04-16T12:00:00Z"),
        );
        state.apply_refresh(refreshed_entries(), projects);

        assert_eq!(
            filtered_project_keys(&state),
            vec![
                "/tmp/project-d",
                "/tmp/project-a",
                "/tmp/project-b",
                "/tmp/project-c",
            ]
        );
    }

    #[test]
    fn apply_refresh_resets_selection_to_first_filtered_result() {
        let projects = sample_projects();
        let mut state = PickerState::new(sample_entries(), projects.clone(), None);
        state.move_down();

        state.apply_refresh(sample_entries(), projects);

        assert_eq!(state.selected_index(), Some(0));
        assert_eq!(
            state.confirm(),
            PickerOutcome::Confirm(sample_entry("c", "window-3"))
        );
    }

    #[test]
    fn apply_refresh_falls_back_to_first_filtered_result_when_selected_project_disappears() {
        let projects = sample_projects();
        let mut state = PickerState::new(sample_entries(), projects.clone(), None);
        state.move_down();

        state.apply_refresh(
            vec![sample_entry("b", "window-2"), sample_entry("c", "window-3")],
            projects,
        );

        assert_eq!(
            state.confirm(),
            PickerOutcome::Confirm(sample_entry("c", "window-3"))
        );
    }

    #[test]
    fn apply_refresh_can_add_new_projects_without_breaking_filtered_ordering() {
        let mut projects = sample_projects();
        let mut state = PickerState::new(sample_entries(), projects.clone(), None);

        projects.insert(
            "/tmp/project-d".to_owned(),
            project_record("2026-04-16T12:00:00Z"),
        );
        state.apply_refresh(refreshed_entries(), projects);

        assert_eq!(
            filtered_project_keys(&state),
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
        let mut state = PickerState::new(sample_entries(), projects.clone(), None);
        state.append_query_char('z');

        state.apply_refresh(sample_entries(), projects);

        assert_eq!(state.selected_index(), None);
        assert_eq!(state.confirm(), PickerOutcome::Cancel);
    }

    #[test]
    fn apply_refresh_searches_against_new_projects() {
        let mut projects = sample_projects();
        let mut state = PickerState::new(sample_entries(), projects.clone(), None);

        projects.insert(
            "/tmp/project-d".to_owned(),
            project_record("2026-04-16T12:00:00Z"),
        );
        state.apply_refresh(refreshed_entries(), projects);

        state.append_query_char('d');

        assert_eq!(
            state.confirm(),
            PickerOutcome::Confirm(sample_entry("d", "window-4"))
        );
    }

    #[test]
    fn apply_refresh_keeps_new_projects_visible_after_clearing_query() {
        let mut projects = sample_projects();
        let mut state = PickerState::new(sample_entries(), projects.clone(), None);

        projects.insert(
            "/tmp/project-d".to_owned(),
            project_record("2026-04-16T12:00:00Z"),
        );
        state.apply_refresh(refreshed_entries(), projects);

        state.append_query_char('d');
        state.pop_query_char();

        assert_eq!(
            filtered_project_keys(&state),
            vec![
                "/tmp/project-d",
                "/tmp/project-c",
                "/tmp/project-a",
                "/tmp/project-b",
            ]
        );
    }

    #[test]
    fn render_shows_empty_state() {
        let mut state = PickerState::new(sample_entries(), sample_projects(), None);
        state.append_query_char('z');

        let buffer = render_buffer(&state, 60, 6);

        assert!(
            buffer_lines(&buffer)
                .iter()
                .any(|line| line.contains("No matches"))
        );
    }

    #[test]
    fn render_shows_refreshing_indicator() {
        let mut state = PickerState::new(sample_entries(), sample_projects(), None);
        state.set_refreshing();

        let buffer = render_buffer(&state, 70, 6);

        assert!(
            buffer_lines(&buffer)
                .iter()
                .any(|line| line.contains("refreshing..."))
        );
    }

    #[test]
    fn render_uses_two_line_layout_on_narrow_widths() {
        let state = PickerState::new(sample_entries(), sample_projects(), None);

        let buffer = render_buffer(&state, 40, 6);
        let lines = buffer_lines(&buffer);

        assert!(lines.iter().any(|line| line.contains("project-c")));
        assert!(lines.iter().any(|line| line.contains("/tmp/project-c")));
    }

    #[test]
    fn render_styles_selected_row_with_custom_background() {
        let state = PickerState::new(sample_entries(), sample_projects(), None);

        let buffer = render_buffer(&state, 70, 6);
        let selected_cell = buffer.cell((2, 1)).expect("selected row cell should exist");

        assert_eq!(selected_cell.bg, SELECTED_ROW_BG);
    }

    fn filtered_project_keys(state: &PickerState) -> Vec<&str> {
        state
            .filtered_projects
            .iter()
            .map(|project_match| project_match.project_key.as_str())
            .collect()
    }

    fn sample_entries() -> Vec<PickerEntry> {
        vec![
            sample_entry("a", "window-1"),
            sample_entry("b", "window-2"),
            sample_entry("c", "window-3"),
        ]
    }

    fn refreshed_entries() -> Vec<PickerEntry> {
        vec![
            sample_entry("a", "window-1"),
            sample_entry("b", "window-2"),
            sample_entry("c", "window-3"),
            sample_entry("d", "window-4"),
        ]
    }

    fn many_entries() -> Vec<PickerEntry> {
        ('a'..='h')
            .enumerate()
            .map(|(index, suffix)| sample_entry(&suffix.to_string(), &format!("window-{index}")))
            .collect()
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

    fn refreshed_projects() -> BTreeMap<String, ProjectStateRecord> {
        let mut projects = sample_projects();
        projects.insert(
            "/tmp/project-d".to_owned(),
            project_record("2026-04-16T12:00:00Z"),
        );
        projects
    }

    fn many_projects() -> BTreeMap<String, ProjectStateRecord> {
        ('a'..='h')
            .enumerate()
            .map(|(offset, suffix)| {
                let hour = 20 - offset as i32;
                (
                    format!("/tmp/project-{suffix}"),
                    project_record(&format!("2026-04-16T{hour:02}:00:00Z")),
                )
            })
            .collect()
    }

    fn sample_entry(suffix: &str, window_id: &str) -> PickerEntry {
        PickerEntry {
            project_key: format!("/tmp/project-{suffix}"),
            window_id: window_id.to_owned(),
            primary_label: format!("project-{suffix}"),
            secondary_path: Some(format!("/tmp/project-{suffix}")),
            window_name: Some("Workspace".to_owned()),
        }
    }

    fn project_record(last_accessed_at: &str) -> ProjectStateRecord {
        ProjectStateRecord {
            last_accessed_at: parse_timestamp(last_accessed_at),
            last_seen_at: parse_timestamp("2026-04-16T12:00:00Z"),
            last_window_id: "window".to_owned(),
            last_window_name: Some("Workspace".to_owned()),
        }
    }

    fn render_buffer(state: &PickerState, width: u16, height: u16) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("test terminal should initialize");
        terminal
            .draw(|frame| render_picker(frame, state))
            .expect("render should succeed");
        terminal.backend().buffer().clone()
    }

    fn buffer_lines(buffer: &Buffer) -> Vec<String> {
        (0..buffer.area.height)
            .map(|y| {
                let mut line = String::new();
                for x in 0..buffer.area.width {
                    line.push_str(buffer[(x, y)].symbol());
                }
                line.trim_end().to_owned()
            })
            .collect()
    }

    fn key_event(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl_key_event(character: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL)
    }

    fn alt_backspace_key_event() -> KeyEvent {
        KeyEvent::new(KeyCode::Backspace, KeyModifiers::ALT)
    }

    fn parse_timestamp(input: &str) -> Timestamp {
        input.parse().expect("timestamp fixture should parse")
    }
}
