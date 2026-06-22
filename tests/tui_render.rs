use runtime_zero::tui_dashboard;
use runtime_zero::tui_render::{render_dashboard, render_dashboard_with_state};
use runtime_zero::tui_state::TuiState;

fn visible_line_width(value: &str) -> usize {
    let mut width = 0;
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for code in chars.by_ref() {
                if code.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            width += 1;
        }
    }
    width
}

#[test]
fn render_plain_dashboard_without_ansi() {
    let rendered = render_dashboard(&tui_dashboard::dashboard(), false);
    assert!(rendered.contains("runtime.zero rz0"));
    assert!(rendered.contains("SCRIPTABLE CLI RAIL"));
    assert!(rendered.contains("rz0 store status"));
    assert!(rendered.contains("read-only · no installs/cleanup/module execution/store writes"));
    assert!(!rendered.contains('…'));
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
fn interactive_color_render_styles_body_without_breaking_text() {
    let rendered = render_dashboard_with_state(
        &tui_dashboard::dashboard(),
        true,
        118,
        30,
        &TuiState::new(4),
    );
    assert!(rendered.contains("\x1b["));
    assert!(rendered.contains("[BLOCKED]"));
    assert!(rendered.contains("DOSSIER 01 · FOUNDATION"));
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

#[test]
fn rendered_frames_keep_visible_width_within_terminal_bounds() {
    let dashboard = tui_dashboard::dashboard();
    let mut state = TuiState::new(dashboard.sections.len());
    state.selected_section = 2;
    state.show_help = true;
    let cases = [
        (40, 12, false),
        (58, 16, false),
        (58, 16, true),
        (80, 20, true),
        (118, 30, true),
        (160, 50, true),
    ];

    for (requested_width, requested_height, color) in cases {
        let rendered = render_dashboard_with_state(
            &dashboard,
            color,
            requested_width,
            requested_height,
            &state,
        );
        let frame_width = usize::from(requested_width).clamp(58, 132);
        let frame_height = usize::from(requested_height).max(16);

        assert!(
            rendered.lines().count() <= frame_height,
            "rendered too many lines for {requested_width}x{requested_height}"
        );
        for line in rendered.lines() {
            assert!(
                visible_line_width(line) <= frame_width,
                "line exceeded visible frame width {frame_width}: {line:?}"
            );
        }
    }
}
