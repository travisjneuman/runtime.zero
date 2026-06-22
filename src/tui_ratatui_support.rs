use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders};

use crate::tui_dashboard::{TuiDashboard, TuiRow, TuiSection};
use crate::tui_state::TuiState;

pub(crate) const COMPACT_HELP_HEIGHT: u16 = 4;
pub(crate) const DEFAULT_HELP_HEIGHT: u16 = 5;

pub(crate) fn nav_line(section: &TuiSection, selected: bool, color: bool) -> Line<'static> {
    let marker = if selected { "▸ " } else { "  " };
    let suffix = if selected { "  active" } else { "" };
    let style = if selected {
        selected_style(color)
    } else {
        Style::default()
    };
    Line::styled(
        format!("{marker}{} {}{suffix}", section.code, section.title),
        style,
    )
}

pub(crate) fn row_line(row: &TuiRow, color: bool) -> Line<'static> {
    label_line(row.label, &row.value, row.tone, color)
}

pub(crate) fn label_line(
    label: &'static str,
    value: &str,
    tone: &'static str,
    color: bool,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<11}"), tone_style(tone, color)),
        Span::raw(value.to_string()),
    ])
}

pub(crate) fn block(title: &'static str, tone: &'static str, color: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(tone_style(tone, color))
        .title(title)
}

pub(crate) fn selected_style(color: bool) -> Style {
    if color {
        Style::default()
            .fg(Color::Indexed(179))
            .bg(Color::Indexed(23))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
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

pub(crate) fn help_height(state: &TuiState, area: Rect) -> u16 {
    if state.show_help {
        DEFAULT_HELP_HEIGHT.min(area.height.saturating_sub(10))
    } else {
        COMPACT_HELP_HEIGHT.min(area.height.saturating_sub(10))
    }
}

pub(crate) fn selected_index(dashboard: &TuiDashboard, state: &TuiState) -> usize {
    if dashboard.sections.is_empty() {
        0
    } else {
        state.selected_section.min(dashboard.sections.len() - 1)
    }
}

pub(crate) fn selected_section<'a>(
    dashboard: &'a TuiDashboard,
    state: &TuiState,
) -> &'a TuiSection {
    &dashboard.sections[selected_index(dashboard, state)]
}
