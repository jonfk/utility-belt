use std::fs;
use std::io::{self, IsTerminal};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use error_stack::{Report, ResultExt};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::{
    CompletedFrame, DefaultTerminal, TerminalOptions, Viewport, try_init_with_options, try_restore,
};

use crate::error::{AppError, AppResult};
use crate::proposal::Proposal;

static UI_ACTIVE: AtomicBool = AtomicBool::new(false);

const MIN_VIEWPORT_HEIGHT: u16 = 12;
const MAX_VIEWPORT_HEIGHT: u16 = 24;
const FILES_RATIO: u16 = 40;
const MESSAGE_RATIO: u16 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewAction {
    Commit,
    Edit,
    Cancel,
    Interrupt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusPane {
    Files,
    Message,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PaneMetrics {
    file_lines: u16,
    file_viewport_height: u16,
    message_lines: u16,
    message_viewport_height: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UiState {
    focus: FocusPane,
    file_scroll: u16,
    message_scroll: u16,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            focus: FocusPane::Message,
            file_scroll: 0,
            message_scroll: 0,
        }
    }
}

pub fn should_use_tui() -> bool {
    should_use_tui_for(
        std::io::stdin().is_terminal(),
        std::io::stdout().is_terminal(),
    )
}

pub fn should_use_tui_for(stdin_is_tty: bool, stdout_is_tty: bool) -> bool {
    stdin_is_tty && stdout_is_tty
}

pub fn restore_terminal_if_active() {
    if UI_ACTIVE.swap(false, Ordering::SeqCst) {
        let _ = try_restore();
    }
}

pub fn review_ready_tui(proposal: &Proposal, message_file: &Path) -> AppResult<ReviewAction> {
    let viewport_height = inline_viewport_height()
        .change_context(AppError::Ui)
        .attach("Failed to determine terminal size for inline UI")?;
    let mut terminal = InlineTerminal::new(viewport_height)?;
    let mut state = UiState::default();

    loop {
        let message = read_message(message_file)?;
        let completed = terminal.draw(|frame| draw_review(frame, proposal, &message, &state))?;
        let metrics = pane_metrics_from_frame(proposal, &message, completed);

        match next_key_event()? {
            Some(key) => {
                if let Some(action) = handle_key_event(&mut state, key, metrics) {
                    return Ok(action);
                }
            }
            None => continue,
        }
    }
}

fn read_message(message_file: &Path) -> AppResult<String> {
    fs::read_to_string(message_file)
        .change_context(AppError::Ui)
        .attach(format!(
            "Failed to read commit message file at {}",
            message_file.display()
        ))
}

fn inline_viewport_height() -> io::Result<u16> {
    let (_, terminal_height) = crossterm::terminal::size()?;
    let capped = terminal_height.saturating_sub(2);
    Ok(capped.clamp(MIN_VIEWPORT_HEIGHT, MAX_VIEWPORT_HEIGHT))
}

fn next_key_event() -> AppResult<Option<KeyEvent>> {
    loop {
        if !event::poll(Duration::from_millis(250))
            .change_context(AppError::Ui)
            .attach("Failed while polling terminal events")?
        {
            return Ok(None);
        }

        match event::read()
            .change_context(AppError::Ui)
            .attach("Failed while reading terminal input")?
        {
            Event::Key(key) if key.kind == KeyEventKind::Press => return Ok(Some(key)),
            Event::Resize(_, _) => return Ok(None),
            _ => continue,
        }
    }
}

fn draw_review(
    frame: &mut ratatui::Frame<'_>,
    proposal: &Proposal,
    message: &str,
    state: &UiState,
) {
    let layout = split_layout(frame.area());
    let title_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let highlight_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let dim_style = Style::default().fg(Color::DarkGray);

    let summary_lines = vec![
        Line::from(vec![
            Span::styled(
                "Ready",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(format!("{} file(s)", proposal.stage_paths.len()), dim_style),
        ]),
        Line::from(proposal.summary.as_str()),
    ];

    let summary = Paragraph::new(summary_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled("Codex Commit", title_style)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(summary, layout.summary);

    let file_lines = if proposal.stage_paths.is_empty() {
        vec![Line::from(Span::styled("(none)", dim_style))]
    } else {
        proposal
            .stage_paths
            .iter()
            .map(|path| Line::from(Span::raw(path.as_str())))
            .collect::<Vec<_>>()
    };

    let files_title = if state.focus == FocusPane::Files {
        Span::styled("Files [focus]", highlight_style)
    } else {
        Span::styled("Files", title_style)
    };
    let files = Paragraph::new(Text::from(file_lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(if state.focus == FocusPane::Files {
                    highlight_style
                } else {
                    Style::default()
                })
                .title(files_title),
        )
        .scroll((state.file_scroll, 0));
    frame.render_widget(files, layout.files);

    let message_lines = normalize_text_lines(message);
    let message_title = if state.focus == FocusPane::Message {
        Span::styled("Commit Message [focus]", highlight_style)
    } else {
        Span::styled("Commit Message", title_style)
    };
    let message_widget = Paragraph::new(Text::from(message_lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(if state.focus == FocusPane::Message {
                    highlight_style
                } else {
                    Style::default()
                })
                .title(message_title),
        )
        .scroll((state.message_scroll, 0));
    frame.render_widget(message_widget, layout.message);

    let footer_text = Line::from(vec![
        Span::styled(
            "Enter",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" commit  "),
        Span::styled(
            "e",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" edit  "),
        Span::styled(
            "q",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" cancel  "),
        Span::styled("Tab", highlight_style),
        Span::raw(" switch pane  "),
        Span::styled("↑↓", highlight_style),
        Span::raw(" scroll"),
    ]);
    let footer = Paragraph::new(footer_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(Span::styled("Actions", title_style)),
    );
    frame.render_widget(footer, layout.footer);
}

fn handle_key_event(
    state: &mut UiState,
    key: KeyEvent,
    metrics: PaneMetrics,
) -> Option<ReviewAction> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('C') => return Some(ReviewAction::Interrupt),
            KeyCode::Char('d') | KeyCode::Char('D') => return Some(ReviewAction::Cancel),
            _ => {}
        }
    }

    match key.code {
        KeyCode::Enter => Some(ReviewAction::Commit),
        KeyCode::Esc => Some(ReviewAction::Cancel),
        KeyCode::Char(ch) if ch.eq_ignore_ascii_case(&'q') => Some(ReviewAction::Cancel),
        KeyCode::Char(ch) if ch.eq_ignore_ascii_case(&'e') => Some(ReviewAction::Edit),
        KeyCode::Char('g') => {
            scroll_active_pane(state, metrics, ScrollDelta::Top);
            None
        }
        KeyCode::Char('G') => {
            scroll_active_pane(state, metrics, ScrollDelta::Bottom);
            None
        }
        KeyCode::Char(ch) if ch.eq_ignore_ascii_case(&'j') => {
            scroll_active_pane(state, metrics, ScrollDelta::Lines(1));
            None
        }
        KeyCode::Char(ch) if ch.eq_ignore_ascii_case(&'k') => {
            scroll_active_pane(state, metrics, ScrollDelta::Lines(-1));
            None
        }
        KeyCode::Tab | KeyCode::Right => {
            state.focus = FocusPane::Message;
            None
        }
        KeyCode::Left => {
            state.focus = FocusPane::Files;
            None
        }
        KeyCode::Up => {
            scroll_active_pane(state, metrics, ScrollDelta::Lines(-1));
            None
        }
        KeyCode::Down => {
            scroll_active_pane(state, metrics, ScrollDelta::Lines(1));
            None
        }
        KeyCode::PageUp => {
            scroll_active_pane(state, metrics, ScrollDelta::PageUp);
            None
        }
        KeyCode::PageDown => {
            scroll_active_pane(state, metrics, ScrollDelta::PageDown);
            None
        }
        KeyCode::Home => {
            scroll_active_pane(state, metrics, ScrollDelta::Top);
            None
        }
        KeyCode::End => {
            scroll_active_pane(state, metrics, ScrollDelta::Bottom);
            None
        }
        _ => None,
    }
}

fn scroll_active_pane(state: &mut UiState, metrics: PaneMetrics, delta: ScrollDelta) {
    match state.focus {
        FocusPane::Files => {
            state.file_scroll = next_scroll(
                state.file_scroll,
                metrics.file_lines,
                metrics.file_viewport_height,
                delta,
            );
        }
        FocusPane::Message => {
            state.message_scroll = next_scroll(
                state.message_scroll,
                metrics.message_lines,
                metrics.message_viewport_height,
                delta,
            );
        }
    }
}

fn pane_metrics_from_frame(
    proposal: &Proposal,
    message: &str,
    completed: CompletedFrame<'_>,
) -> PaneMetrics {
    let layout = split_layout(completed.area);
    PaneMetrics {
        file_lines: proposal.stage_paths.len().max(1) as u16,
        file_viewport_height: inner_height(layout.files),
        message_lines: normalize_text_lines(message).len().max(1) as u16,
        message_viewport_height: inner_height(layout.message),
    }
}

fn split_layout(area: Rect) -> ReviewLayout {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(6),
            Constraint::Length(3),
        ])
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(FILES_RATIO),
            Constraint::Percentage(MESSAGE_RATIO),
        ])
        .split(vertical[1]);

    ReviewLayout {
        summary: vertical[0],
        files: horizontal[0],
        message: horizontal[1],
        footer: vertical[2],
    }
}

fn inner_height(area: Rect) -> u16 {
    area.height.saturating_sub(2).max(1)
}

fn normalize_text_lines(text: &str) -> Vec<Line<'static>> {
    let lines = text
        .lines()
        .map(|line| Line::from(line.to_string()))
        .collect::<Vec<_>>();
    if lines.is_empty() {
        vec![Line::from(String::new())]
    } else {
        lines
    }
}

fn next_scroll(current: u16, content_lines: u16, viewport_lines: u16, delta: ScrollDelta) -> u16 {
    let max_scroll = content_lines.saturating_sub(viewport_lines);
    let next = match delta {
        ScrollDelta::Lines(change) if change.is_negative() => {
            current.saturating_sub(change.unsigned_abs())
        }
        ScrollDelta::Lines(change) => current.saturating_add(change as u16),
        ScrollDelta::PageUp => current.saturating_sub(viewport_lines.max(1)),
        ScrollDelta::PageDown => current.saturating_add(viewport_lines.max(1)),
        ScrollDelta::Top => 0,
        ScrollDelta::Bottom => max_scroll,
    };
    next.min(max_scroll)
}

struct InlineTerminal {
    terminal: DefaultTerminal,
}

impl InlineTerminal {
    fn new(viewport_height: u16) -> AppResult<Self> {
        let terminal = try_init_with_options(TerminalOptions {
            viewport: Viewport::Inline(viewport_height),
        })
        .map_err(Report::from)
        .change_context(AppError::Ui)
        .attach(format!(
            "Failed to initialize inline terminal viewport with height {viewport_height}"
        ));

        match terminal {
            Ok(terminal) => {
                UI_ACTIVE.store(true, Ordering::SeqCst);
                Ok(Self { terminal })
            }
            Err(report) => {
                let _ = try_restore();
                Err(report)
            }
        }
    }

    fn draw<F>(&mut self, render: F) -> AppResult<CompletedFrame<'_>>
    where
        F: FnOnce(&mut ratatui::Frame<'_>),
    {
        self.terminal
            .draw(render)
            .change_context(AppError::Ui)
            .attach("Failed to render terminal UI frame")
    }
}

impl Drop for InlineTerminal {
    fn drop(&mut self) {
        restore_terminal_if_active();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScrollDelta {
    Lines(i16),
    PageUp,
    PageDown,
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReviewLayout {
    summary: Rect,
    files: Rect,
    message: Rect,
    footer: Rect,
}

#[cfg(test)]
#[path = "ui_tests.rs"]
mod tests;
