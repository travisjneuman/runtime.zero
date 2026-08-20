use ratatui::style::{Color, Modifier, Style};

use crate::tui_dashboard::{TuiDashboard, TuiSection};

pub(crate) fn selected_index(
    dashboard: &TuiDashboard,
    state: &crate::tui_state::TuiState,
) -> usize {
    if dashboard.sections.is_empty() {
        0
    } else {
        state.selected_section.min(dashboard.sections.len() - 1)
    }
}

pub(crate) fn selected_section<'a>(
    dashboard: &'a TuiDashboard,
    state: &crate::tui_state::TuiState,
) -> &'a TuiSection {
    &dashboard.sections[selected_index(dashboard, state)]
}

pub(crate) fn selected_row_index(
    section: &TuiSection,
    state: &crate::tui_state::TuiState,
) -> usize {
    if section.rows.is_empty() {
        0
    } else {
        state.selected_detail_row.min(section.rows.len() - 1)
    }
}

pub(crate) fn selected_style(color: bool) -> Style {
    if color {
        Style::default()
            .fg(Color::Indexed(179))
            .bg(Color::Indexed(23))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED)
    }
}

pub(crate) fn strong_style(color: bool) -> Style {
    if color {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

pub(crate) fn tone_style(tone: &str, color: bool) -> Style {
    if !color {
        return Style::default();
    }
    Style::default().fg(match tone {
        "accent" => Color::Indexed(179),
        "info" => Color::Indexed(110),
        "safe" => Color::Indexed(108),
        "dry_run" => Color::Indexed(147),
        "warn" => Color::Indexed(179),
        "muted" => Color::Indexed(245),
        _ => Color::Reset,
    })
}
