use ratatui::Terminal;
use ratatui::backend::TestBackend;
use runtime_zero::tui_layout::TuiLayoutTier;
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
    assert!(text.contains("LOCAL CONTROL"));
    assert!(text.contains("HOME / NEXT STEP"));
    assert!(text.contains(tui_theme::LABEL_OK));
    assert!(text.contains(tui_theme::LABEL_INFO));
    assert!(text.contains("SELECTED"));
}

#[test]
fn monitor_shortcut_renders_live_resource_rows() {
    let dashboard = runtime_zero::tui_dashboard::dashboard();
    let mut state = TuiState::new(dashboard.sections.len());
    state.apply(runtime_zero::tui_state::TuiInput::OpenMonitor);
    let text = render_text(118, 34, &state, false);
    assert!(text.contains("SYSTEM"));
    assert!(text.contains("memory"));
    assert!(text.contains("processes"));
}

#[test]
fn selected_section_changes_detail_panel() {
    let dashboard = runtime_zero::tui_dashboard::dashboard();
    let mut state = TuiState::new(dashboard.sections.len());
    state.selected_section = 2;
    let text = render_text(110, 32, &state, false);
    assert!(text.contains("SOFTWARE"));
    assert!(text.contains("SOFTWARE"));
}

#[test]
fn compact_frame_renders_safe_notice_without_panic() {
    let text = render_text(42, 10, &TuiState::new(4), false);
    assert!(text.contains("TERMINAL TOO SMALL"));
    assert!(text.contains("rz0 --no-tui"));
}

#[test]
fn compact_layout_keeps_focus_and_safety_visible() {
    let mut state = TuiState::new(4);
    state.apply(runtime_zero::tui_state::TuiInput::FocusNext);
    let text = render_text(58, 16, &state, false);
    assert!(text.contains("HOME / NEXT STEP"));
    assert!(text.contains("SELECTED"));
    assert!(text.contains("↑↓ move"));
    assert!(text.contains("q quit"));
}

#[test]
fn layout_tiers_are_named_and_bounded() {
    assert_eq!(TuiLayoutTier::from_size(42, 10), TuiLayoutTier::VerySmall);
    assert_eq!(TuiLayoutTier::from_size(58, 16), TuiLayoutTier::Compact);
    assert_eq!(TuiLayoutTier::from_size(90, 24), TuiLayoutTier::Standard);
    assert_eq!(TuiLayoutTier::from_size(120, 34), TuiLayoutTier::Wide);
    assert_eq!(TuiLayoutTier::Wide.name(), "wide");
}

#[test]
fn help_mode_preserves_cli_escape_hatch_copy() {
    let mut state = TuiState::new(4);
    state.show_help = true;
    let text = render_text(90, 24, &state, false);
    assert!(text.contains("Tab / Shift+Tab"));
    assert!(text.contains("Esc              close this view"));
    assert!(text.contains("q quits"));
}

#[test]
fn focus_regions_are_visible_without_color() {
    let mut state = TuiState::new(4);
    let text = render_text(110, 32, &state, false);
    assert!(text.contains("HOME"));

    state.apply(runtime_zero::tui_state::TuiInput::FocusNext);
    let details = render_text(110, 32, &state, false);
    assert!(details.contains("HOME / NEXT STEP"));
    assert!(details.contains("SELECTED"));

    state.apply(runtime_zero::tui_state::TuiInput::FocusNext);
    let context = render_text(110, 32, &state, false);
    assert!(context.contains("NEXT ACTION"));
    assert!(context.contains("No command has run"));
}

#[test]
fn read_only_previews_do_not_claim_execution() {
    let mut state = TuiState::new(4);
    state.apply(runtime_zero::tui_state::TuiInput::FocusNext);
    state.apply(runtime_zero::tui_state::TuiInput::FocusNext);
    state.apply(runtime_zero::tui_state::TuiInput::Activate);
    let text = render_text(110, 32, &state, false);
    assert!(text.contains("NEXT ACTION"));
    assert!(text.contains("Details"));
    assert!(!text.contains("installed successfully"));
}

#[test]
fn live_software_command_is_previewed_without_execution_claims() {
    let mut state = TuiState::new(4);
    state.apply(runtime_zero::tui_state::TuiInput::FocusNext);
    state.apply(runtime_zero::tui_state::TuiInput::FocusNext);
    state.apply(runtime_zero::tui_state::TuiInput::Activate);
    let text = render_text(118, 34, &state, false);
    assert!(text.contains("NEXT ACTION"));
    assert!(text.contains("Details"));
    assert!(!text.contains("QUICK COMMANDS"));
}

#[test]
fn polished_shell_uses_component_labels_without_color_dependency() {
    let text = render_text(118, 34, &TuiState::new(4), false);
    assert!(text.contains("runtime.zero"));
    assert!(text.contains("LOCAL CONTROL"));
    assert!(text.contains("HOME / NEXT STEP"));
    assert!(text.contains("SELECTED"));
    assert!(text.contains("status"));
}

#[test]
fn color_mode_does_not_change_required_text_labels() {
    let state = TuiState::new(4);
    let plain = render_text(110, 32, &state, false);
    let color = render_text(110, 32, &state, true);
    for label in [tui_theme::LABEL_OK, tui_theme::LABEL_INFO] {
        assert!(plain.contains(label));
        assert!(color.contains(label));
    }
    let dashboard = runtime_zero::tui_dashboard::dashboard();
    let mut modules = TuiState::new(dashboard.sections.len());
    modules.selected_section = 3;
    assert!(render_text(110, 32, &modules, false).contains(tui_theme::LABEL_INFO));
    assert!(render_text(110, 32, &modules, true).contains(tui_theme::LABEL_INFO));
}

#[test]
fn bottom_selection_stays_visible_and_enter_opens_details() {
    let dashboard = runtime_zero::tui_dashboard::dashboard();
    let mut state = TuiState::new(dashboard.sections.len());
    state.apply(runtime_zero::tui_state::TuiInput::FocusNext);
    state.selected_section = 2;
    state.selected_detail_row = usize::MAX;
    let text = render_text(110, 32, &state, false);
    let count = dashboard.sections[2].rows.len();
    assert!(count > 0);
    assert!(text.contains("▶"));

    state.apply(runtime_zero::tui_state::TuiInput::Activate);
    let details = render_text(110, 32, &state, false);
    assert!(details.contains("Details"));
    assert!(details.contains("source:"));
}

#[test]
fn ratatui_frame_keeps_terminal_boundaries_across_sizes() {
    for (width, height) in [(42, 10), (58, 16), (80, 24), (120, 34)] {
        let text = render_text(width, height, &TuiState::new(4), false);
        assert_eq!(text.lines().count(), usize::from(height));
        for line in text.lines() {
            assert!(line.chars().count() <= usize::from(width));
        }
    }
}
