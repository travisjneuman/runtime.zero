use ratatui::backend::Backend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::{Frame, Terminal};

use crate::tui_dashboard::TuiDashboard;
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
    if area.width < 50 || area.height < 12 {
        render_compact_notice(frame, area, color);
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

    render_header(frame, vertical[0], dashboard, color);
    render_body(frame, vertical[1], dashboard, state, color);
    render_help(frame, vertical[2], state, color);
    render_footer(frame, vertical[3], color);
}

fn render_compact_notice(frame: &mut Frame<'_>, area: Rect, color: bool) {
    let lines = vec![
        Line::styled("runtime.zero", tone_style("accent", color)),
        Line::raw("safe review dashboard"),
        Line::raw("Terminal too small for the full TUI."),
        Line::raw("Use rz0 --no-tui or resize wider/taller."),
        Line::raw("q/Esc exits when interactive."),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(block("COMPACT", "info", color)),
        area,
    );
}

fn render_header(frame: &mut Frame<'_>, area: Rect, dashboard: &TuiDashboard, color: bool) {
    let lines = vec![
        Line::from(vec![
            Span::styled(dashboard.title, tone_style("accent", color)),
            Span::raw("  "),
            Span::styled(dashboard.command, tone_style("info", color)),
            Span::raw(format!("  v{}", dashboard.version)),
        ]),
        Line::from(vec![
            Span::styled("foundation control surface", tone_style("info", color)),
            Span::raw(" · "),
            Span::raw(dashboard.mode),
            Span::raw(" · local-first"),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(block("RUNTIME DOSSIER", "accent", color)),
        area,
    );
}

fn render_body(
    frame: &mut Frame<'_>,
    area: Rect,
    dashboard: &TuiDashboard,
    state: &TuiState,
    color: bool,
) {
    if area.width >= WIDE_LAYOUT_WIDTH {
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
        Line::styled(section.summary, tone_style("info", color)),
        Line::raw(format!(
            "section {} / {} · read-only",
            selected_index(dashboard, state) + 1,
            dashboard.sections.len()
        )),
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
        lines.push(Line::styled(
            format!("PREVIEW · {} {}", row.label, row.value),
            tone_style("accent", color),
        ));
        lines.push(Line::raw("read-only context only; no action will run"));
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

fn render_state_cards(frame: &mut Frame<'_>, area: Rect, dashboard: &TuiDashboard, color: bool) {
    let lines = vec![
        Line::from(vec![
            Span::styled("store ", tone_style("info", color)),
            Span::raw(format!("{:?}", dashboard.store_init_status).to_lowercase()),
            Span::raw("   "),
            Span::styled("registry ", tone_style("info", color)),
            Span::raw(format!("{:?}", dashboard.registry_state).to_lowercase()),
        ]),
        Line::from(vec![
            Span::styled("receipts ", tone_style("info", color)),
            Span::raw(format!("{:?}", dashboard.receipt_state).to_lowercase()),
            Span::raw("   "),
            Span::styled("modules ", tone_style("info", color)),
            Span::raw(format!("{} installed", dashboard.installed_module_count)),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(block("FOUNDATION STATE / live", "info", color)),
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

fn render_footer(frame: &mut Frame<'_>, area: Rect, color: bool) {
    let line = Line::styled(
        "read-only · no installs/cleanup/module execution/store writes",
        tone_style("dry_run", color),
    );
    frame.render_widget(
        Paragraph::new(vec![line]).block(block("SAFETY", "dry_run", color)),
        area,
    );
}
