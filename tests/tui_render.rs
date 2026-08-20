use runtime_zero::tui_dashboard;
use runtime_zero::tui_render::{render_dashboard, render_dashboard_with_state};
use runtime_zero::tui_state::TuiState;
use runtime_zero::updates::{LiveUpdateCatalog, UPDATE_CATALOG_CONTRACT};

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

fn strip_ansi(value: &str) -> String {
    let mut output = String::new();
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
            output.push(ch);
        }
    }
    output
}

#[test]
fn render_plain_dashboard_without_ansi() {
    let rendered = render_dashboard(&tui_dashboard::dashboard(), false);
    assert!(rendered.contains("runtime.zero"));
    assert!(rendered.contains("LOCAL CONTROL"));
    assert!(rendered.contains("HOME / NEXT STEP"));
    assert!(rendered.contains("status"));
    assert!(!rendered.contains("\x1b["));
}

#[test]
fn render_wide_dashboard_has_navigation_and_selected_section() {
    let mut state = TuiState::new(4);
    state.selected_section = 1;
    let rendered = render_dashboard_with_state(&tui_dashboard::dashboard(), false, 118, 30, &state);
    assert!(rendered.contains("TOOLCHAIN"));
    assert!(rendered.contains("TOOLCHAIN"));
    assert!(rendered.contains("Rust-first AI and developer toolchain records"));
    assert!(rendered.contains("SELECTED"));
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
    assert!(rendered.contains("[INFO]"));
    assert!(rendered.contains("HOME / NEXT STEP"));
}

#[test]
fn render_handles_narrow_terminal_and_help() {
    let mut state = TuiState::new(4);
    state.show_help = true;
    let rendered = render_dashboard_with_state(&tui_dashboard::dashboard(), false, 40, 16, &state);
    assert!(rendered.contains("Esc"));
    assert!(rendered.contains("HELP"));
    assert!(!rendered.contains("\x1b["));
}

#[test]
fn update_check_status_is_visible_in_compact_interactive_frames() {
    let mut dashboard = tui_dashboard::dashboard();
    let state = TuiState::new(dashboard.sections.len());

    assert!(dashboard.start_update_check().is_some());
    dashboard.apply_software_view(state.software_view());
    let checking = render_dashboard_with_state(&dashboard, false, 80, 24, &state);
    assert!(checking.contains("checking provider availability"));
    assert!(
        dashboard
            .update_action_status
            .contains("waiting for results")
    );

    dashboard.complete_update_check(LiveUpdateCatalog {
        schema_version: 1,
        contract: UPDATE_CATALOG_CONTRACT,
        checked: true,
        read_only: true,
        writes_attempted: false,
        network_read_requested: true,
        source_count: 5,
        source_ok_count: 3,
        candidate_count: 2,
        candidates: Vec::new(),
        warnings: Vec::new(),
    });
    dashboard.apply_software_view(state.software_view());
    let checked = render_dashboard_with_state(&dashboard, false, 80, 24, &state);
    assert!(checked.contains("checked · 2 candidates · 3/5 sources"));
    assert!(
        dashboard
            .update_action_status
            .contains("review ready · choose Review action")
    );

    assert!(dashboard.start_update_check().is_some());
    dashboard.fail_update_check();
    dashboard.apply_software_view(state.software_view());
    let failed = render_dashboard_with_state(&dashboard, false, 80, 24, &state);
    assert!(failed.contains("update check failed · press u to retry"));
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

#[test]
fn all_sections_render_with_accessible_labels_across_terminal_sizes() {
    let dashboard = tui_dashboard::dashboard();
    let sizes = [(58, 16), (80, 20), (118, 30), (160, 50)];

    for selected_section in 0..dashboard.sections.len() {
        for show_help in [false, true] {
            for (requested_width, requested_height) in sizes {
                for color in [false, true] {
                    let mut state = TuiState::new(dashboard.sections.len());
                    state.selected_section = selected_section;
                    state.show_help = show_help;
                    let rendered = render_dashboard_with_state(
                        &dashboard,
                        color,
                        requested_width,
                        requested_height,
                        &state,
                    );
                    let plain = strip_ansi(&rendered);
                    let frame_width = usize::from(requested_width).clamp(58, 132);
                    let frame_height = usize::from(requested_height).max(16);
                    let section = &dashboard.sections[selected_section];

                    assert!(
                        rendered.lines().count() <= frame_height,
                        "rendered too many lines for section {} at {requested_width}x{requested_height}",
                        section.code
                    );
                    for line in rendered.lines() {
                        assert!(
                            visible_line_width(line) <= frame_width,
                            "line exceeded visible frame width {frame_width}: {line:?}"
                        );
                    }
                    assert!(plain.contains("runtime.zero"));
                    assert!(plain.contains("runtime.zero"));
                    if !show_help || requested_height >= 24 {
                        let expected_section_title = if section.title == "overview" {
                            "HOME / NEXT STEP"
                        } else {
                            &section.title.to_uppercase()
                        };
                        assert!(
                            plain.contains(expected_section_title) || plain.contains(section.title)
                        );
                    }
                    if show_help {
                        assert!(plain.contains("Esc"));
                        assert!(plain.contains("Tab / Shift+Tab"));
                    } else {
                        assert!(plain.contains("Tab focus"));
                    }
                }
            }
        }
    }
}

#[test]
fn colorized_frames_preserve_plain_text_contract() {
    let dashboard = tui_dashboard::dashboard();
    let mut state = TuiState::new(dashboard.sections.len());
    state.selected_section = 4;
    state.show_help = true;

    let plain = render_dashboard_with_state(&dashboard, false, 118, 30, &state);
    let colorized = render_dashboard_with_state(&dashboard, true, 118, 30, &state);

    assert!(colorized.contains("\x1b["));
    assert_eq!(strip_ansi(&colorized), plain);
    assert!(plain.contains("[INFO]"));
    assert!(plain.contains("[PLAN]"));
}
