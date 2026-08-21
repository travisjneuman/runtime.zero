use crate::tui_canvas::{border_bottom, border_top, line, line_plain, separator, truncate};
use crate::tui_dashboard::{TuiDashboard, TuiRow, TuiSection, WORKSPACE_LABELS, workspace_heading};
use crate::tui_theme;

const TEXT_WIDTH: usize = 86;
const MIN_WIDTH: usize = 58;
const MAX_WIDTH: usize = 132;

pub fn render_dashboard(dashboard: &TuiDashboard, color: bool) -> String {
    render_dashboard_frame(dashboard, color, TEXT_WIDTH, 38)
}

fn render_dashboard_frame(
    dashboard: &TuiDashboard,
    color: bool,
    requested_width: usize,
    requested_height: usize,
) -> String {
    let width = requested_width.clamp(MIN_WIDTH, MAX_WIDTH);
    let height = requested_height.max(16);
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
            snapshot_heading(dashboard),
            dashboard.installed_software_count,
            dashboard.installed_module_count
        ),
        width,
        color,
        Some(tui_theme::TuiTone::Info),
    ));
    lines.push(separator(width));
    lines.push(line(
        &workspace_tabs(dashboard),
        width,
        color,
        Some(tui_theme::TuiTone::Accent),
    ));
    lines.push(separator(width));

    let section = dashboard.sections.first();
    let tail = vec![
        separator(width),
        line(
            &status_line(dashboard),
            width,
            color,
            Some(tui_theme::TuiTone::Info),
        ),
        line_plain(&key_line(width), width),
    ];

    let prefix_budget = height.saturating_sub(tail.len() + 1);
    let selected = selected_lines(section, width, color);
    let primary_budget = prefix_budget.saturating_sub(lines.len() + 1 + selected.len());
    if let Some(section) = section {
        lines.extend(primary_lines(section, width, color, primary_budget));
    }
    lines.push(separator(width));
    lines.extend(selected);
    lines.truncate(prefix_budget);
    lines.extend(tail);
    lines.push(border_bottom(width));
    lines.join("\n") + "\n"
}

fn snapshot_heading(dashboard: &TuiDashboard) -> &str {
    if dashboard.inventory_status == "loading" {
        "loading local snapshot"
    } else if dashboard.inventory_status.starts_with("unavailable") {
        "local snapshot unavailable"
    } else {
        "local snapshot"
    }
}

fn workspace_tabs(_dashboard: &TuiDashboard) -> String {
    let names = WORKSPACE_LABELS;
    let current = 0;
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

fn primary_lines(section: &TuiSection, width: usize, color: bool, max_lines: usize) -> Vec<String> {
    let title = workspace_heading(section.title);
    let mut lines = vec![line(&title, width, color, Some(tui_theme::TuiTone::Accent))];
    lines.push(line(
        &format!("{} · {} items", section.summary, section.rows.len()),
        width,
        color,
        Some(tui_theme::TuiTone::Muted),
    ));
    let selected = 0;
    if section.rows.is_empty() {
        lines.push(line_plain(
            "No records are available in this workspace.",
            width,
        ));
    } else {
        let row_budget = max_lines.saturating_sub(lines.len());
        for (index, row) in section.rows.iter().take(row_budget).enumerate() {
            lines.push(line(
                &format_row(row, index == selected, width),
                width,
                color,
                tone_for_row(row),
            ));
        }
    }
    lines
}

fn selected_lines(section: Option<&TuiSection>, width: usize, color: bool) -> Vec<String> {
    let Some(section) = section else {
        return vec![line(
            "Selected",
            width,
            color,
            Some(tui_theme::TuiTone::Accent),
        )];
    };
    let Some(row) = section.rows.first() else {
        return vec![line(
            "Selected",
            width,
            color,
            Some(tui_theme::TuiTone::Accent),
        )];
    };
    let explanation = row
        .preview
        .clone()
        .unwrap_or_else(|| format!("{}: {}", row.label, row.value));
    vec![
        line("Selected", width, color, Some(tui_theme::TuiTone::Accent)),
        line(
            &format!("{}  {}", row.label, row.value),
            width,
            color,
            tone_for_row(row),
        ),
        line_plain(&truncate(&explanation, width.saturating_sub(4)), width),
    ]
}

fn status_line(dashboard: &TuiDashboard) -> String {
    dashboard.update_action_status.clone()
}

fn key_line(width: usize) -> String {
    if width < 82 {
        "↑↓ move · Tab focus · Enter details · ? help · q quit".to_string()
    } else {
        "commands: rz0 doctor · rz0 apps · rz0 store status · rz0 --json".to_string()
    }
}

fn format_row(row: &TuiRow, selected: bool, width: usize) -> String {
    let marker = if selected { "· " } else { "  " };
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
