use ratatui::backend::Backend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::{Frame, Terminal};

use crate::tui_dashboard::TuiDashboard;
use crate::tui_layout::TuiLayoutTier;
use crate::tui_ratatui_components::{
    preview_only_line, render_compact_dashboard, render_compact_notice, render_footer,
    render_header, render_state_cards,
};
use crate::tui_ratatui_rail::render_command_rail;
use crate::tui_ratatui_support::{
    block, focus_summary, focused_title, help_height, label_line, nav_line, row_line,
    selectable_row_line, selected_index, selected_row_index, selected_section, strong_style,
    tone_style,
};
use crate::tui_state::{TuiFocusRegion, TuiState};
use crate::tui_theme;

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
    let mut lines = vec![Line::styled("NAVIGATION", tone_style("info", color))];
    for (index, section) in dashboard.sections.iter().enumerate() {
        lines.push(nav_line(section, index == current, color));
    }
    lines.push(Line::raw(""));
    lines.push(Line::styled("POSTURE", tone_style("info", color)));
    lines.push(label_line(
        tui_theme::LABEL_INFO,
        "safe review",
        "info",
        color,
    ));
    lines.push(label_line(
        tui_theme::LABEL_BLOCKED,
        "module execution blocked",
        "warn",
        color,
    ));
    frame.render_widget(
        Paragraph::new(lines).block(block(
            focused_title(
                "INDEX",
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
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                format!("DOSSIER {} · ", section.code),
                tone_style("accent", color),
            ),
            Span::styled(section.title.to_uppercase(), strong_style(color)),
        ]),
        Line::from(vec![
            Span::styled(section.summary, tone_style("info", color)),
            Span::raw("   "),
            Span::styled(
                format!(
                    "section {} / {}",
                    selected_index(dashboard, state) + 1,
                    dashboard.sections.len()
                ),
                tone_style("muted", color),
            ),
        ]),
        Line::styled(
            focus_summary(state.focus_region),
            tone_style("muted", color),
        ),
        Line::raw(""),
    ];
    for (index, row) in section.rows.iter().enumerate() {
        let focused = state.focus_region == TuiFocusRegion::DetailsPanel && index == selected_row;
        if state.focus_region == TuiFocusRegion::DetailsPanel {
            lines.push(selectable_row_line(row, focused, color));
        } else {
            lines.push(row_line(row, color));
        }
    }
    if state.preview_open && state.focus_region == TuiFocusRegion::DetailsPanel {
        let row = &section.rows[selected_row];
        lines.push(Line::raw(""));
        lines.push(preview_only_line(color));
        lines.push(Line::raw(format!("context: {} {}", row.label, row.value)));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(block(
                focused_title(
                    "SELECTED SECTION",
                    state.focus_region == TuiFocusRegion::DetailsPanel,
                ),
                "accent",
                color,
            ))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_help(frame: &mut Frame<'_>, area: Rect, state: &TuiState, color: bool) {
    let lines = if state.show_help {
        vec![
            Line::raw("Tab/Shift+Tab focus · ↑/↓/j/k move within focus · Enter/Space preview"),
            Line::raw("Esc closes preview/help or backs out · q quits"),
            Line::raw("subcommands, --json, pipes, and --no-tui stay CLI-only"),
        ]
    } else {
        vec![Line::raw(
            "keys: Tab focus · ↑/↓/j/k move · Enter preview · h help · Esc back · q quit",
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
