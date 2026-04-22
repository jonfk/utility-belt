use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{
    FocusPane, PaneMetrics, ReviewAction, ScrollDelta, UiState, handle_key_event, next_scroll,
    should_use_tui_for,
};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl_key(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL)
}

fn metrics() -> PaneMetrics {
    PaneMetrics {
        file_lines: 20,
        file_viewport_height: 5,
        message_lines: 30,
        message_viewport_height: 6,
    }
}

#[test]
fn tty_detection_requires_both_streams() {
    assert!(should_use_tui_for(true, true));
    assert!(!should_use_tui_for(true, false));
    assert!(!should_use_tui_for(false, true));
}

#[test]
fn tab_switches_focus_to_message_pane() {
    let mut state = UiState {
        focus: FocusPane::Files,
        file_scroll: 0,
        message_scroll: 0,
    };

    let result = handle_key_event(&mut state, key(KeyCode::Tab), metrics());

    assert_eq!(result, None);
    assert_eq!(state.focus, FocusPane::Message);
}

#[test]
fn left_switches_focus_to_files_pane() {
    let mut state = UiState::default();

    let result = handle_key_event(&mut state, key(KeyCode::Left), metrics());

    assert_eq!(result, None);
    assert_eq!(state.focus, FocusPane::Files);
}

#[test]
fn down_scrolls_the_focused_message_pane() {
    let mut state = UiState::default();

    handle_key_event(&mut state, key(KeyCode::Down), metrics());

    assert_eq!(state.message_scroll, 1);
    assert_eq!(state.file_scroll, 0);
}

#[test]
fn page_down_is_clamped_to_content_height() {
    let scroll = next_scroll(0, 4, 10, ScrollDelta::PageDown);

    assert_eq!(scroll, 0);
}

#[test]
fn action_keys_map_to_review_outcomes() {
    let mut state = UiState::default();

    assert_eq!(
        handle_key_event(&mut state, key(KeyCode::Enter), metrics()),
        Some(ReviewAction::Commit)
    );
    assert_eq!(
        handle_key_event(&mut state, key(KeyCode::Char('e')), metrics()),
        Some(ReviewAction::Edit)
    );
    assert_eq!(
        handle_key_event(&mut state, key(KeyCode::Esc), metrics()),
        Some(ReviewAction::Cancel)
    );
}

#[test]
fn ctrl_c_maps_to_interrupt() {
    let mut state = UiState::default();

    assert_eq!(
        handle_key_event(&mut state, ctrl_key('c'), metrics()),
        Some(ReviewAction::Interrupt)
    );
}

#[test]
fn ctrl_d_maps_to_cancel() {
    let mut state = UiState::default();

    assert_eq!(
        handle_key_event(&mut state, ctrl_key('d'), metrics()),
        Some(ReviewAction::Cancel)
    );
}

#[test]
fn j_and_k_scroll_the_focused_pane() {
    let mut state = UiState::default();

    handle_key_event(&mut state, key(KeyCode::Char('j')), metrics());
    assert_eq!(state.message_scroll, 1);

    handle_key_event(&mut state, key(KeyCode::Char('k')), metrics());
    assert_eq!(state.message_scroll, 0);
}

#[test]
fn home_and_end_jump_to_scroll_boundaries() {
    let max_scroll = next_scroll(0, 30, 6, ScrollDelta::Bottom);
    assert_eq!(next_scroll(7, 30, 6, ScrollDelta::Top), 0);
    assert_eq!(max_scroll, 24);
}

#[test]
fn g_and_uppercase_g_jump_to_top_and_bottom() {
    let mut state = UiState::default();

    handle_key_event(&mut state, key(KeyCode::Char('G')), metrics());
    assert_eq!(state.message_scroll, 24);

    handle_key_event(&mut state, key(KeyCode::Char('g')), metrics());
    assert_eq!(state.message_scroll, 0);
}
