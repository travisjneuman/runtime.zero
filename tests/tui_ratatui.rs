use ratatui::Terminal;
use ratatui::backend::TestBackend;
use runtime_zero::tui_ratatui::draw_dashboard;
use runtime_zero::tui_state::TuiState;
use runtime_zero::tui_theme;

fn render_text(width: u16, height: u16, state: &TuiState, color: bool) -> String {
    let dashboard = runtime_zero::tui_dashboard::dashboard();
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    draw_dashboard(&mut terminal, &dashboard, state, color).expect("draw");
    let buffer = terminal.backend().buffer();
    let area = buffer.area;
    let mut text = String::new();
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            text.push_str(buffer[(x, y)].symbol());
        }
        text.push('\n');
    }
    text
}

#[test]
fn widget_dashboard_keeps_text_first_labels() {
    let text = render_text(110, 32, &TuiState::new(4), false);
    assert!(text.contains("runtime.zero"));
    assert!(text.contains("NAVIGATION"));
    assert!(text.contains("DOSSIER 01"));
    assert!(text.contains(tui_theme::LABEL_OK));
    assert!(text.contains(tui_theme::LABEL_BLOCKED));
    assert!(text.contains("SCRIPTABLE CLI RAIL"));
}

#[test]
fn selected_section_changes_detail_panel() {
    let mut state = TuiState::new(4);
    state.selected_section = 2;
    let text = render_text(110, 32, &state, false);
    assert!(text.contains("DOSSIER 03"));
    assert!(text.contains("module planning without activation"));
}

#[test]
fn compact_frame_renders_safe_notice_without_panic() {
    let text = render_text(42, 10, &TuiState::new(4), false);
    assert!(text.contains("Terminal too small"));
    assert!(text.contains("rz0 --no-tui"));
}

#[test]
fn help_mode_preserves_cli_escape_hatch_copy() {
    let mut state = TuiState::new(4);
    state.show_help = true;
    let text = render_text(90, 24, &state, false);
    assert!(text.contains("--json"));
    assert!(text.contains("--no-tui"));
    assert!(text.contains("Esc closes preview/help"));
    assert!(text.contains("q quits"));
}

#[test]
fn focus_regions_are_visible_without_color() {
    let mut state = TuiState::new(4);
    let text = render_text(110, 32, &state, false);
    assert!(text.contains("INDEX [FOCUS]"));

    state.apply(runtime_zero::tui_state::TuiInput::FocusNext);
    let details = render_text(110, 32, &state, false);
    assert!(details.contains("SELECTED SECTION [FOCUS]"));
    assert!(details.contains("▶ [OK]"));

    state.apply(runtime_zero::tui_state::TuiInput::FocusNext);
    let rail = render_text(110, 32, &state, false);
    assert!(rail.contains("SCRIPTABLE CLI RAIL [FOCUS]"));
    assert!(rail.contains("▶ [INFO]"));
    assert!(rail.contains("doctor"));
}

#[test]
fn read_only_previews_do_not_claim_execution() {
    let mut state = TuiState::new(4);
    state.apply(runtime_zero::tui_state::TuiInput::FocusNext);
    state.apply(runtime_zero::tui_state::TuiInput::FocusNext);
    state.apply(runtime_zero::tui_state::TuiInput::Activate);
    let text = render_text(110, 32, &state, false);
    assert!(text.contains("PREVIEW"));
    assert!(text.contains("no command execution from TUI"));
    assert!(!text.contains("installed successfully"));
}

#[test]
fn polished_shell_uses_component_labels_without_color_dependency() {
    let text = render_text(118, 34, &TuiState::new(4), false);
    assert!(text.contains("RZ0 // FOUNDATION CONTROL SURFACE"));
    assert!(text.contains("FOUNDATION STATE // LIVE"));
    assert!(text.contains("SAFETY // LOCKED"));
    assert!(text.contains("preview/control surface only"));
    assert!(text.contains("select to preview; TUI will not run commands"));
}

#[test]
fn color_mode_does_not_change_required_text_labels() {
    let state = TuiState::new(4);
    let plain = render_text(110, 32, &state, false);
    let color = render_text(110, 32, &state, true);
    for label in [
        tui_theme::LABEL_OK,
        tui_theme::LABEL_INFO,
        tui_theme::LABEL_BLOCKED,
    ] {
        assert!(plain.contains(label));
        assert!(color.contains(label));
    }
    let mut modules = TuiState::new(4);
    modules.selected_section = 2;
    assert!(render_text(110, 32, &modules, false).contains(tui_theme::LABEL_DRY_RUN));
    assert!(render_text(110, 32, &modules, true).contains(tui_theme::LABEL_DRY_RUN));
}

#[test]
fn ratatui_frame_keeps_terminal_boundaries_across_sizes() {
    for (width, height) in [(58, 16), (80, 24), (120, 34)] {
        let text = render_text(width, height, &TuiState::new(4), false);
        assert_eq!(text.lines().count(), usize::from(height));
        for line in text.lines() {
            assert!(line.chars().count() <= usize::from(width));
        }
    }
}
