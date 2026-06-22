use crate::tui_canvas::{pad, truncate};
use crate::tui_dashboard::{TuiDashboard, TuiRow, TuiSection};
use crate::tui_state::TuiState;
use crate::tui_theme;

pub(crate) fn selected_section<'a>(
    dashboard: &'a TuiDashboard,
    state: &TuiState,
) -> &'a TuiSection {
    &dashboard.sections[selected_index(dashboard, state)]
}

pub(crate) fn selected_index(dashboard: &TuiDashboard, state: &TuiState) -> usize {
    if dashboard.sections.is_empty() {
        0
    } else {
        state.selected_section.min(dashboard.sections.len() - 1)
    }
}

pub(crate) fn format_row(row: &TuiRow, width: usize) -> String {
    let value_width = width.saturating_sub(24);
    format!("{:<11} {}", row.label, truncate(&row.value, value_width))
}

pub(crate) fn card_line(
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

pub(crate) fn humanize_debug(value: &str) -> String {
    let mut output = String::new();
    for (index, ch) in value.chars().enumerate() {
        if index > 0 && ch.is_uppercase() {
            output.push(' ');
        }
        output.extend(ch.to_lowercase());
    }
    output
}

pub(crate) fn tone_for_text(value: &str) -> Option<tui_theme::TuiTone> {
    let trimmed = value.trim_start();
    if trimmed.starts_with('▸')
        || trimmed.starts_with("DOSSIER")
        || trimmed.starts_with("SCRIPTABLE")
    {
        return Some(tui_theme::TuiTone::Accent);
    }
    if trimmed.starts_with("NAVIGATION")
        || trimmed.starts_with("POSTURE")
        || trimmed.starts_with("FOUNDATION STATE")
        || trimmed.contains(tui_theme::LABEL_INFO)
    {
        return Some(tui_theme::TuiTone::Info);
    }
    if trimmed.contains(tui_theme::LABEL_OK) {
        return Some(tui_theme::TuiTone::Safe);
    }
    if trimmed.contains(tui_theme::LABEL_DRY_RUN) {
        return Some(tui_theme::TuiTone::DryRun);
    }
    if trimmed.contains(tui_theme::LABEL_PLAN) {
        return Some(tui_theme::TuiTone::Accent);
    }
    if trimmed.contains(tui_theme::LABEL_WARN) || trimmed.contains(tui_theme::LABEL_BLOCKED) {
        return Some(tui_theme::TuiTone::Warn);
    }
    if trimmed.contains(tui_theme::LABEL_SKIP) || trimmed.ends_with("read-only") {
        return Some(tui_theme::TuiTone::Muted);
    }
    None
}
