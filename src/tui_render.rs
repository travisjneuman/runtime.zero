use crate::tui_canvas::{
    border_bottom, border_top, line, line_plain, pad, separator, split_line, truncate,
};
use crate::tui_dashboard::{TuiDashboard, TuiRow, TuiSection};
use crate::tui_state::TuiState;
use crate::tui_theme;

const TEXT_WIDTH: usize = 86;
const MIN_WIDTH: usize = 58;
const MAX_WIDTH: usize = 132;
const NAV_WIDTH: usize = 24;

pub fn render_dashboard(dashboard: &TuiDashboard, color: bool) -> String {
    render_dashboard_frame(
        dashboard,
        color,
        TEXT_WIDTH as u16,
        34,
        &TuiState::new(0),
        false,
    )
}

pub fn render_dashboard_with_state(
    dashboard: &TuiDashboard,
    color: bool,
    width: u16,
    height: u16,
    state: &TuiState,
) -> String {
    render_dashboard_frame(dashboard, color, width, height, state, true)
}

fn render_dashboard_frame(
    dashboard: &TuiDashboard,
    color: bool,
    width: u16,
    height: u16,
    state: &TuiState,
    interactive: bool,
) -> String {
    let width = usize::from(width).clamp(MIN_WIDTH, MAX_WIDTH);
    let height = usize::from(height).max(16);
    let mut lines = Vec::new();
    lines.extend(header_lines(dashboard, width, color));
    lines.extend(body_lines(dashboard, state, width, height, interactive));
    lines.extend(footer_lines(state, width, interactive, color));
    lines.push(border_bottom(width));
    lines.join("\n") + "\n"
}

fn header_lines(dashboard: &TuiDashboard, width: usize, color: bool) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(border_top(width));
    lines.push(line(
        &format!(
            "{} {}  v{}",
            dashboard.title, dashboard.command, dashboard.version
        ),
        width,
        color,
        Some(tui_theme::TuiTone::Accent),
    ));
    lines.push(line(
        "foundation control surface · safe review dashboard · local-first",
        width,
        color,
        Some(tui_theme::TuiTone::Info),
    ));
    lines.push(separator(width));
    lines
}

fn body_lines(
    dashboard: &TuiDashboard,
    state: &TuiState,
    width: usize,
    height: usize,
    interactive: bool,
) -> Vec<String> {
    let budget = height.saturating_sub(if state.show_help { 12 } else { 9 });
    if width >= 92 {
        wide_body_lines(dashboard, state, width, budget, interactive)
    } else {
        compact_body_lines(dashboard, state, width, budget, interactive)
    }
}

fn wide_body_lines(
    dashboard: &TuiDashboard,
    state: &TuiState,
    width: usize,
    budget: usize,
    interactive: bool,
) -> Vec<String> {
    let right_width = width.saturating_sub(NAV_WIDTH + 5);
    let mut left = navigation_lines(dashboard, state, interactive);
    let mut right = selected_panel_lines(dashboard, state, right_width);
    right.extend(status_card_lines(dashboard, right_width));
    right.extend(command_rail_lines(right_width));
    pad_columns(&mut left, &mut right, budget);
    left.into_iter()
        .zip(right)
        .map(|(l, r)| split_line(&l, &r, NAV_WIDTH, right_width))
        .collect()
}

fn compact_body_lines(
    dashboard: &TuiDashboard,
    state: &TuiState,
    width: usize,
    budget: usize,
    interactive: bool,
) -> Vec<String> {
    let inner = width.saturating_sub(4);
    let mut content = navigation_lines(dashboard, state, interactive);
    content.push(String::new());
    content.extend(selected_panel_lines(dashboard, state, inner));
    content.extend(status_card_lines(dashboard, inner));
    content.extend(command_rail_lines(inner));
    content.truncate(budget.max(1));
    content
        .into_iter()
        .map(|value| line_plain(&value, width))
        .collect()
}

fn navigation_lines(dashboard: &TuiDashboard, state: &TuiState, interactive: bool) -> Vec<String> {
    let mut lines = vec!["NAVIGATION".to_string()];
    for (index, section) in dashboard.sections.iter().enumerate() {
        let selected = interactive && index == selected_index(dashboard, state);
        let marker = if selected { "▸" } else { " " };
        let suffix = if selected { "  active" } else { "" };
        lines.push(format!(
            "{marker} {} {}{}",
            section.code, section.title, suffix
        ));
    }
    lines.push(String::new());
    lines.push("POSTURE".to_string());
    lines.push(format!("{} safe review", tui_theme::LABEL_INFO));
    lines.push(format!(
        "{} module execution blocked",
        tui_theme::LABEL_BLOCKED
    ));
    lines
}

fn selected_panel_lines(dashboard: &TuiDashboard, state: &TuiState, width: usize) -> Vec<String> {
    let section = selected_section(dashboard, state);
    let mut lines = vec![format!(
        "DOSSIER {} · {}",
        section.code,
        section.title.to_uppercase()
    )];
    lines.push(truncate(section.summary, width));
    lines.push(format!(
        "section {} / {} · read-only",
        selected_index(dashboard, state) + 1,
        dashboard.sections.len()
    ));
    lines.push(String::new());
    for row in &section.rows {
        lines.push(format_row(row, width));
    }
    lines
}

fn status_card_lines(dashboard: &TuiDashboard, width: usize) -> Vec<String> {
    let mut lines = vec![String::new(), "FOUNDATION STATE / live".to_string()];
    lines.push(card_line(
        "store",
        &humanize_debug(&format!("{:?}", dashboard.store_init_status)),
        "registry",
        &humanize_debug(&format!("{:?}", dashboard.registry_state)),
        width,
    ));
    lines.push(card_line(
        "receipts",
        &humanize_debug(&format!("{:?}", dashboard.receipt_state)),
        "modules",
        &format!("{} installed", dashboard.installed_module_count),
        width,
    ));
    lines
}

fn command_rail_lines(width: usize) -> Vec<String> {
    vec![
        String::new(),
        "SCRIPTABLE CLI RAIL".to_string(),
        truncate("rz0 doctor · rz0 store status · rz0 --json", width),
        truncate("rz0 modules install --dry-run <package>", width),
    ]
}

fn footer_lines(state: &TuiState, width: usize, interactive: bool, color: bool) -> Vec<String> {
    let mut lines = vec![separator(width)];
    lines.push(line(
        "read-only · no installs · no cleanup · no module execution · no store writes from TUI",
        width,
        color,
        Some(tui_theme::TuiTone::DryRun),
    ));
    if interactive && state.show_help {
        lines.push(line_plain(
            "keys: q/Esc quit · h/? help · ↑/↓/j/k/Tab navigate · Home/End jump",
            width,
        ));
        lines.push(line_plain(
            "automation: subcommands, --json, pipes, and --no-tui stay CLI-only",
            width,
        ));
    } else if interactive {
        lines.push(line_plain(
            "keys: q quit · h help · ↑/↓/j/k navigate · Home/End jump",
            width,
        ));
    } else {
        lines.push(line_plain(
            "commands: rz0 doctor · rz0 store status · rz0 store init --dry-run",
            width,
        ));
    }
    lines
}

fn selected_section<'a>(dashboard: &'a TuiDashboard, state: &TuiState) -> &'a TuiSection {
    &dashboard.sections[selected_index(dashboard, state)]
}

fn selected_index(dashboard: &TuiDashboard, state: &TuiState) -> usize {
    if dashboard.sections.is_empty() {
        0
    } else {
        state.selected_section.min(dashboard.sections.len() - 1)
    }
}

fn pad_columns(left: &mut Vec<String>, right: &mut Vec<String>, budget: usize) {
    left.truncate(budget);
    right.truncate(budget);
    let target = left.len().max(right.len()).max(1);
    left.resize(target, String::new());
    right.resize(target, String::new());
}

fn format_row(row: &TuiRow, width: usize) -> String {
    let value_width = width.saturating_sub(24);
    format!("{:<11} {}", row.label, truncate(&row.value, value_width))
}

fn card_line(
    left_label: &str,
    left_value: &str,
    right_label: &str,
    right_value: &str,
    width: usize,
) -> String {
    let half = width.saturating_sub(3) / 2;
    let left = format!("{}: {}", left_label, left_value.to_lowercase());
    let right = format!("{}: {}", right_label, right_value.to_lowercase());
    format!(
        "{} │ {}",
        pad(&truncate(&left.to_lowercase(), half), half),
        truncate(&right.to_lowercase(), half)
    )
}

fn humanize_debug(value: &str) -> String {
    let mut output = String::new();
    for (index, ch) in value.chars().enumerate() {
        if index > 0 && ch.is_uppercase() {
            output.push(' ');
        }
        output.extend(ch.to_lowercase());
    }
    output
}
