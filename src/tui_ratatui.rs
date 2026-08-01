use ratatui::backend::Backend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::{Frame, Terminal};

use crate::tui_dashboard::TuiDashboard;
use crate::tui_layout::TuiLayoutTier;
use crate::tui_ratatui_components::{
    render_compact_dashboard, render_compact_notice, render_footer, render_header,
    render_state_cards,
};
use crate::tui_ratatui_rail::render_command_rail;
use crate::tui_ratatui_support::{
    block, focused_title, help_height, nav_line, row_line, selectable_row_line, selected_index,
    selected_row_index, selected_section, strong_style, tone_style,
};
use crate::tui_state::{TuiFocusRegion, TuiState};

const MIN_NAV_WIDTH: u16 = 26;
const WIDE_LAYOUT_WIDTH: u16 = 92;

pub fn draw_dashboard<B: Backend>(
    terminal: &mut Terminal<B>,
    dashboard: &TuiDashboard,
    state: &TuiState,
    color: bool,
) -> Result<(), B::Error> {
    terminal
        .draw(|frame| render_dashboard(frame, dashboard, state, color))
        .map(|_| ())
}

fn render_dashboard(
    frame: &mut Frame<'_>,
    dashboard: &TuiDashboard,
    state: &TuiState,
    color: bool,
) {
    let area = frame.area();
    let tier = TuiLayoutTier::from_size(area.width, area.height);

    if tier == TuiLayoutTier::VerySmall {
        render_compact_notice(frame, area, tier, color);
        return;
    }
    if tier == TuiLayoutTier::Compact {
        render_compact_dashboard(frame, area, dashboard, state, tier, color);
        return;
    }
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(8),
            Constraint::Length(help_height(state, area)),
            Constraint::Length(3),
        ])
        .split(area);

    render_header(frame, vertical[0], dashboard, tier, color);
    render_body(frame, vertical[1], dashboard, state, tier, color);
    render_help(frame, vertical[2], state, color);
    render_footer(frame, vertical[3], color);
}

fn render_body(
    frame: &mut Frame<'_>,
    area: Rect,
    dashboard: &TuiDashboard,
    state: &TuiState,
    tier: TuiLayoutTier,
    color: bool,
) {
    if tier == TuiLayoutTier::Wide || area.width >= WIDE_LAYOUT_WIDTH {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(MIN_NAV_WIDTH), Constraint::Min(40)])
            .split(area);
        render_navigation(frame, chunks[0], dashboard, state, color);
        render_detail_stack(frame, chunks[1], dashboard, state, color);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Min(8)])
            .split(area);
        render_navigation(frame, chunks[0], dashboard, state, color);
        render_detail_stack(frame, chunks[1], dashboard, state, color);
    }
}

fn render_navigation(
    frame: &mut Frame<'_>,
    area: Rect,
    dashboard: &TuiDashboard,
    state: &TuiState,
    color: bool,
) {
    let current = selected_index(dashboard, state);
    let mut lines = vec![Line::styled("SECTIONS", tone_style("info", color))];
    for (index, section) in dashboard.sections.iter().enumerate() {
        lines.push(nav_line(section, index == current, color));
    }
    frame.render_widget(
        Paragraph::new(lines).block(block(
            focused_title(
                "SECTIONS",
                state.focus_region == TuiFocusRegion::LeftNavigation,
            ),
            "accent",
            color,
        )),
        area,
    );
}

fn render_detail_stack(
    frame: &mut Frame<'_>,
    area: Rect,
    dashboard: &TuiDashboard,
    state: &TuiState,
    color: bool,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(4),
            Constraint::Length(7),
        ])
        .split(area);
    render_selected_panel(frame, chunks[0], dashboard, state, color);
    render_state_cards(frame, chunks[1], dashboard, color);
    render_command_rail(frame, chunks[2], state, color);
}

fn render_selected_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    dashboard: &TuiDashboard,
    state: &TuiState,
    color: bool,
) {
    let section = selected_section(dashboard, state);
    let selected_row = selected_row_index(section, state);
    let show_details = state.preview_open
        && state.focus_region == TuiFocusRegion::DetailsPanel
        && section.rows.get(selected_row).is_some()
        && area.height >= 10;
    if show_details {
        let preview_height = 4.min(area.height.saturating_sub(6));
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(preview_height)])
            .split(area);
        render_selected_list(frame, chunks[0], dashboard, state, color);
        if let Some(row) = section.rows.get(selected_row) {
            render_selected_details(frame, chunks[1], row, color);
        }
    } else {
        render_selected_list(frame, area, dashboard, state, color);
    }
}

fn render_selected_list(
    frame: &mut Frame<'_>,
    area: Rect,
    dashboard: &TuiDashboard,
    state: &TuiState,
    color: bool,
) {
    let section = selected_section(dashboard, state);
    let selected_row = selected_row_index(section, state);
    let header_lines = vec![
        Line::from(vec![
            Span::styled(section.title.to_uppercase(), strong_style(color)),
            Span::raw("   "),
            Span::styled(
                format!(
                    "{} of {}",
                    selected_index(dashboard, state) + 1,
                    dashboard.sections.len()
                ),
                tone_style("muted", color),
            ),
        ]),
        Line::from(vec![Span::styled(
            section.summary,
            tone_style("info", color),
        )]),
        Line::styled(
            if section.rows.is_empty() {
                "item 0 of 0".to_string()
            } else {
                format!("item {} of {}", selected_row + 1, section.rows.len())
            },
            tone_style("muted", color),
        ),
    ];
    let mut rows = Vec::with_capacity(section.rows.len());
    for (index, row) in section.rows.iter().enumerate() {
        let selected = state.focus_region == TuiFocusRegion::DetailsPanel && index == selected_row;
        if state.focus_region == TuiFocusRegion::DetailsPanel {
            rows.push(selectable_row_line(row, selected, color));
        } else {
            rows.push(row_line(row, color));
        }
    }
    let panel = block(section.title.to_uppercase(), "accent", color);
    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    let header_height = 3.min(inner.height);
    let header_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: header_height,
    };
    frame.render_widget(Paragraph::new(header_lines), header_area);
    let list_area = Rect {
        x: inner.x,
        y: inner.y.saturating_add(header_height),
        width: inner.width,
        height: inner.height.saturating_sub(header_height),
    };
    let list_height = usize::from(list_area.height);
    let max_scroll = rows.len().saturating_sub(list_height);
    let scroll = selected_row
        .saturating_sub(list_height.saturating_sub(1))
        .min(max_scroll);
    frame.render_widget(
        Paragraph::new(rows).scroll((u16::try_from(scroll).unwrap_or(u16::MAX), 0)),
        list_area,
    );
}

fn render_selected_details(
    frame: &mut Frame<'_>,
    area: Rect,
    row: &crate::tui_dashboard::TuiRow,
    color: bool,
) {
    let details = row
        .preview
        .clone()
        .unwrap_or_else(|| format!("{}: {}", row.label, row.value));
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(row.label, tone_style(row.tone, color)),
                Span::raw(" "),
                Span::raw(row.value.clone()),
            ]),
            Line::raw(details),
        ])
        .block(block("DETAILS", "info", color))
        .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_help(frame: &mut Frame<'_>, area: Rect, state: &TuiState, color: bool) {
    let lines = if state.search_active() {
        vec![Line::raw(format!(
            "search: {} · type to filter · Backspace edit · Enter accept · Esc cancel",
            state.search_query()
        ))]
    } else if state.show_help {
        vec![
            Line::raw("Tab/Shift+Tab areas · ↑/↓/j/k move · mouse wheel scrolls · Enter details"),
            Line::raw("u check updates · / search · f filter · s sort · r refresh · Esc back"),
            Line::raw("q quit · h or ? close this help"),
        ]
    } else {
        vec![Line::raw(
            "Tab areas · ↑/↓/j/k move · mouse wheel scrolls · Enter details · u updates · q quit",
        )]
    };
    frame.render_widget(
        Paragraph::new(lines).block(block(
            focused_title("KEYS", state.focus_region == TuiFocusRegion::HelpOverlay),
            "info",
            color,
        )),
        area,
    );
}
