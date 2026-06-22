use runtime_zero::tui_dashboard;
use runtime_zero::tui_render::{render_dashboard, render_dashboard_with_state};
use runtime_zero::tui_state::TuiState;

#[test]
fn render_plain_dashboard_without_ansi() {
    let rendered = render_dashboard(&tui_dashboard::dashboard(), false);
    assert!(rendered.contains("runtime.zero rz0"));
    assert!(rendered.contains("SCRIPTABLE CLI RAIL"));
    assert!(rendered.contains("rz0 store status"));
    assert!(!rendered.contains("\x1b["));
}

#[test]
fn render_wide_dashboard_has_navigation_and_selected_section() {
    let mut state = TuiState::new(4);
    state.selected_section = 1;
    let rendered = render_dashboard_with_state(&tui_dashboard::dashboard(), false, 118, 30, &state);
    assert!(rendered.contains("NAVIGATION"));
    assert!(rendered.contains("▸ 02 local store"));
    assert!(rendered.contains("DOSSIER 02 · LOCAL STORE"));
    assert!(rendered.contains("user-local store and registry health"));
    assert!(rendered.contains("FOUNDATION STATE"));
}

#[test]
fn render_handles_narrow_terminal_and_help() {
    let mut state = TuiState::new(4);
    state.show_help = true;
    let rendered = render_dashboard_with_state(&tui_dashboard::dashboard(), false, 40, 16, &state);
    assert!(rendered.contains("q/Esc quit"));
    assert!(rendered.contains("NAVIGATION"));
    assert!(!rendered.contains("\x1b["));
}
