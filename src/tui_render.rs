use crate::tui_canvas::{border_bottom, border_top, line, line_plain, separator, truncate};
use crate::tui_dashboard::{TuiDashboard, TuiRow, TuiSection, WORKSPACE_LABELS, workspace_heading};
use crate::tui_render_support::{selected_index, selected_section};
use crate::tui_state::{TuiFocusRegion, TuiState};
use crate::tui_theme;

const TEXT_WIDTH: usize = 86;
const MIN_WIDTH: usize = 58;
const MAX_WIDTH: usize = 132;

pub fn render_dashboard(dashboard: &TuiDashboard, color: bool) -> String {
    render_dashboard_frame(
        dashboard,
        color,
        TEXT_WIDTH as u16,
        38,
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
    lines.push(border_top(width));
    lines.push(line(
        "runtime.zero · local snapshot",
        width,
        color,
        Some(tui_theme::TuiTone::Accent),
    ));
    lines.push(line(
        &format!(
            "{} · {} software · {} modules · review first",
            if dashboard.inventory_status == "loading" {
                "loading local snapshot"
            } else {
                "local snapshot"
            },
            dashboard.installed_software_count,
            dashboard.installed_module_count
        ),
        width,
        color,
        Some(tui_theme::TuiTone::Info),
    ));
    lines.push(separator(width));
    lines.push(line(
        &workspace_tabs(dashboard, state),
        width,
        color,
        Some(tui_theme::TuiTone::Accent),
    ));
    lines.push(separator(width));

    if state.show_help && height < 24 {
        lines.push(line("Help", width, color, Some(tui_theme::TuiTone::Info)));
        lines.push(line_plain(
            "Tab / Shift+Tab focus · arrows/j/k move · Enter details",
            width,
        ));
        lines.push(line_plain("Esc closes help · q quits", width));
        lines.push(border_bottom(width));
        return lines.join("\n") + "\n";
    }

    let section = selected_section(dashboard, state);
    lines.extend(primary_lines(section, state, width, color));
    lines.push(separator(width));
    lines.extend(selected_lines(dashboard, state, width, color));
    let mut tail = vec![
        separator(width),
        line(
            &status_line(dashboard, state),
            width,
            color,
            Some(tui_theme::TuiTone::Info),
        ),
        line_plain(&key_line(state, interactive, width), width),
    ];
    if state.show_help {
        tail.push(separator(width));
        tail.push(line_plain("Help", width));
        tail.push(line_plain(
            "Tab / Shift+Tab focus · arrows/j/k move · Enter details · Esc closes · q quits",
            width,
        ));
        tail.push(line_plain(
            "u review providers · Review action [U] · r refresh · m system · / search",
            width,
        ));
    } else if state.search_active() {
        tail.push(line_plain(
            &format!(
                "Search · query: {} · type · Backspace edit · Enter accepts · Esc cancels",
                state.search_query()
            ),
            width,
        ));
    } else if state.update_confirmation_active() {
        tail.push(line_plain(
            &format!(
                "Confirm one action · entered: {} · Enter applies · Esc cancels",
                state.update_confirmation_phrase()
            ),
            width,
        ));
    }

    let prefix_budget = height.saturating_sub(tail.len() + 1);
    lines.truncate(prefix_budget);
    lines.extend(tail);
    lines.push(border_bottom(width));
    lines.join("\n") + "\n"
}

fn workspace_tabs(dashboard: &TuiDashboard, state: &TuiState) -> String {
    let names = WORKSPACE_LABELS;
    let current = selected_index(dashboard, state);
    names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            if index == current {
                format!("• {name} •")
            } else {
                format!("  {name}  ")
            }
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

fn primary_lines(section: &TuiSection, state: &TuiState, width: usize, color: bool) -> Vec<String> {
    let title = workspace_heading(section.title);
    let mut lines = vec![line(&title, width, color, Some(tui_theme::TuiTone::Accent))];
    lines.push(line(
        &format!("{} · {} items", section.summary, section.rows.len()),
        width,
        color,
        Some(tui_theme::TuiTone::Muted),
    ));
    let selected = if section.rows.is_empty() {
        0
    } else {
        state.selected_detail_row.min(section.rows.len() - 1)
    };
    if section.rows.is_empty() {
        lines.push(line_plain(
            "No records are available in this workspace.",
            width,
        ));
    } else {
        for (index, row) in section.rows.iter().enumerate() {
            lines.push(line(
                &format_row(row, index == selected, state, width),
                width,
                color,
                tone_for_row(row),
            ));
        }
    }
    lines
}

fn selected_lines(
    dashboard: &TuiDashboard,
    state: &TuiState,
    width: usize,
    color: bool,
) -> Vec<String> {
    let context_focus = state.focus_region == TuiFocusRegion::ContextPane;
    let section = selected_section(dashboard, state);
    let selected = if section.rows.is_empty() {
        0
    } else {
        state.selected_detail_row.min(section.rows.len() - 1)
    };
    let Some(row) = section.rows.get(selected) else {
        return vec![line(
            if context_focus {
                "Next action"
            } else {
                "Selected"
            },
            width,
            color,
            Some(if context_focus {
                tui_theme::TuiTone::Info
            } else {
                tui_theme::TuiTone::Accent
            }),
        )];
    };
    let explanation = if state.preview_open {
        row.preview
            .clone()
            .unwrap_or_else(|| format!("{}: {}", row.label, row.value))
    } else if context_focus {
        "Review action [U]: inspect provider evidence before confirmation. No command has run."
            .to_string()
    } else {
        "Press Enter for source, state, and the exact next step. Nothing has run.".to_string()
    };
    vec![
        line(
            if context_focus {
                "Next action"
            } else {
                "Selected"
            },
            width,
            color,
            Some(if context_focus {
                tui_theme::TuiTone::Info
            } else {
                tui_theme::TuiTone::Accent
            }),
        ),
        line(
            &format!("{}  {}", row.label, row.value),
            width,
            color,
            tone_for_row(row),
        ),
        line_plain(&truncate(&explanation, width.saturating_sub(4)), width),
    ]
}

fn status_line(dashboard: &TuiDashboard, state: &TuiState) -> String {
    if state.update_confirmation_active() {
        "confirmation required · type the exact phrase in the dialog".to_string()
    } else if state.search_active() {
        "search active · type to filter · Enter accepts · Esc cancels".to_string()
    } else if state.show_help {
        "help open · Esc closes".to_string()
    } else {
        dashboard.update_action_status.clone()
    }
}

fn key_line(state: &TuiState, interactive: bool, width: usize) -> String {
    if !interactive {
        return "commands: rz0 doctor · rz0 apps · rz0 store status · rz0 --json".to_string();
    }
    if state.search_active() {
        "search input active".to_string()
    } else if state.show_help {
        "Esc closes help".to_string()
    } else if width < 82 {
        "↑↓ move · Tab focus · Enter details · ? help · q quit".to_string()
    } else {
        "↑↓/jk move · Tab focus · Enter details · u check · Review action [U] · r refresh · ? help · q quit"
            .to_string()
    }
}

fn format_row(row: &TuiRow, selected: bool, state: &TuiState, width: usize) -> String {
    let marker = if selected && state.focus_region == TuiFocusRegion::DetailsPanel {
        "▶ "
    } else if selected {
        "· "
    } else {
        "  "
    };
    let value_width = width.saturating_sub(20);
    format!(
        "{marker}{:<12} {}",
        row.label,
        truncate(&row.value, value_width)
    )
}

fn tone_for_row(row: &TuiRow) -> Option<tui_theme::TuiTone> {
    match row.tone {
        "safe" => Some(tui_theme::TuiTone::Safe),
        "info" => Some(tui_theme::TuiTone::Info),
        "accent" => Some(tui_theme::TuiTone::Accent),
        "dry_run" => Some(tui_theme::TuiTone::DryRun),
        "warn" => Some(tui_theme::TuiTone::Warn),
        _ => Some(tui_theme::TuiTone::Muted),
    }
}
