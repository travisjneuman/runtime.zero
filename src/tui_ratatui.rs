use ratatui::backend::Backend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::{Frame, Terminal};

use crate::tui_dashboard::{TuiDashboard, TuiRow, TuiSection};
use crate::tui_layout::TuiLayoutTier;
use crate::tui_ratatui_support::{
    selected_index, selected_row_index, selected_section, selected_style, strong_style, tone_style,
};
use crate::tui_state::{TuiFocusRegion, TuiState};

const WIDE_DETAIL_WIDTH: u16 = 38;
const COMPACT_DETAIL_HEIGHT: u16 = 6;

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
        render_small_notice(frame, area, color);
        return;
    }

    let chrome = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(6),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    render_header(frame, chrome[0], dashboard, color);
    render_workspace_tabs(frame, chrome[1], dashboard, state, color);
    render_workspace(frame, chrome[2], dashboard, state, tier, color);
    render_status(frame, chrome[3], dashboard, state, color);
    render_keys(frame, chrome[4], area.width, color);

    if state.show_help {
        render_help_modal(frame, area, color);
    } else if state.update_confirmation_active() {
        render_confirmation_modal(frame, area, dashboard, state, color);
    } else if state.search_active() {
        render_search_modal(frame, area, state, color);
    }
}

fn render_small_notice(frame: &mut Frame<'_>, area: Rect, color: bool) {
    let lines = vec![
        Line::styled("runtime.zero", tone_style("accent", color)),
        Line::raw("The interactive view needs a larger terminal."),
        Line::raw("Resize to at least 50x12 or use:"),
        Line::styled("rz0 --no-tui", tone_style("info", color)),
        Line::raw("q / Esc  exit"),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(panel_block("TERMINAL TOO SMALL", "info", color))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_header(frame: &mut Frame<'_>, area: Rect, dashboard: &TuiDashboard, color: bool) {
    let readiness = if dashboard.inventory_status == "loading" {
        ("loading local snapshot", "accent")
    } else if dashboard.inventory_status.starts_with("unavailable") {
        ("inventory unavailable", "warn")
    } else if dashboard.update_check_status == "not checked" {
        ("ready · updates not checked", "info")
    } else {
        (dashboard.update_check_status.as_str(), "safe")
    };
    let lines = vec![
        Line::from(vec![
            Span::styled(
                "runtime.zero",
                tone_style("accent", color).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ·  LOCAL SNAPSHOT", tone_style("muted", color)),
            Span::raw("  "),
            Span::styled(readiness.0, tone_style(readiness.1, color)),
        ]),
        Line::styled(
            ellipsize(
                &format!(
                    "{} software  ·  {} modules  ·  review before action",
                    dashboard.installed_software_count, dashboard.installed_module_count
                ),
                area.width,
            ),
            tone_style("muted", color),
        ),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}

fn render_workspace_tabs(
    frame: &mut Frame<'_>,
    area: Rect,
    dashboard: &TuiDashboard,
    state: &TuiState,
    color: bool,
) {
    let current = selected_index(dashboard, state);
    let names = ["HOME", "TOOLCHAIN", "SOFTWARE", "SYSTEM", "DIAGNOSTICS"];
    let content = if area.width < 72 {
        format!(
            "workspace {}/{}  ·  {}  ·  Tab changes focus",
            current + 1,
            dashboard.sections.len(),
            names.get(current).copied().unwrap_or("HOME")
        )
    } else {
        names
            .iter()
            .enumerate()
            .map(|(index, name)| {
                if index == current {
                    format!("[ {name} ]")
                } else {
                    format!("  {name}  ")
                }
            })
            .collect::<Vec<_>>()
            .join("")
    };
    let style = if state.focus_region == TuiFocusRegion::LeftNavigation {
        selected_style(color)
    } else {
        strong_style(color)
    };
    frame.render_widget(
        Paragraph::new(Line::styled(content, style)).block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(tone_style("muted", color)),
        ),
        area,
    );
}

fn render_workspace(
    frame: &mut Frame<'_>,
    area: Rect,
    dashboard: &TuiDashboard,
    state: &TuiState,
    tier: TuiLayoutTier,
    color: bool,
) {
    if tier == TuiLayoutTier::Compact || area.width < 82 {
        let detail_height =
            if state.preview_open || state.focus_region == TuiFocusRegion::ContextPane {
                COMPACT_DETAIL_HEIGHT.min(area.height.saturating_sub(5))
            } else {
                4.min(area.height.saturating_sub(5))
            };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(detail_height)])
            .split(area);
        render_primary_panel(frame, chunks[0], dashboard, state, color);
        render_selected_panel(frame, chunks[1], dashboard, state, color);
        return;
    }

    let detail_width = if tier == TuiLayoutTier::Wide {
        WIDE_DETAIL_WIDTH
    } else {
        32
    };
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(36), Constraint::Length(detail_width)])
        .split(area);
    render_primary_panel(frame, chunks[0], dashboard, state, color);
    render_selected_panel(frame, chunks[1], dashboard, state, color);
}

fn render_primary_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    dashboard: &TuiDashboard,
    state: &TuiState,
    color: bool,
) {
    let section = selected_section(dashboard, state);
    let title = workspace_title(section);
    let panel = panel_block(title, "accent", color);
    let inner = panel.inner(area);
    frame.render_widget(panel, area);

    let summary = Line::styled(
        ellipsize(
            &format!("{}  ·  {} items", section.summary, section.rows.len()),
            inner.width,
        ),
        tone_style("muted", color),
    );
    let summary_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: 1.min(inner.height),
    };
    frame.render_widget(Paragraph::new(summary), summary_area);

    let list_area = Rect {
        x: inner.x,
        y: inner.y.saturating_add(summary_area.height),
        width: inner.width,
        height: inner.height.saturating_sub(summary_area.height),
    };
    let selected = selected_row_index(section, state);
    let rows = if section.rows.is_empty() {
        vec![Line::styled(
            "No records are available in this workspace.",
            tone_style("muted", color),
        )]
    } else {
        section
            .rows
            .iter()
            .enumerate()
            .map(|(index, row)| workspace_row(row, index == selected, state, color, inner.width))
            .collect()
    };
    let visible_height = usize::from(list_area.height.max(1));
    let max_scroll = rows.len().saturating_sub(visible_height);
    let scroll = selected
        .saturating_sub(visible_height.saturating_sub(1))
        .min(max_scroll);
    frame.render_widget(
        Paragraph::new(rows).scroll((u16::try_from(scroll).unwrap_or(u16::MAX), 0)),
        list_area,
    );
}

fn render_selected_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    dashboard: &TuiDashboard,
    state: &TuiState,
    color: bool,
) {
    let focused = state.focus_region == TuiFocusRegion::ContextPane;
    let title = if focused { "NEXT ACTION" } else { "SELECTED" };
    let tone = if focused { "info" } else { "accent" };
    let panel = panel_block(title, tone, color);
    let inner = panel.inner(area);
    frame.render_widget(panel, area);

    let section = selected_section(dashboard, state);
    let selected = selected_row_index(section, state);
    let Some(row) = section.rows.get(selected) else {
        frame.render_widget(
            Paragraph::new("Select a workspace item to see its explanation.")
                .wrap(Wrap { trim: true }),
            inner,
        );
        return;
    };
    let explanation = if state.preview_open {
        row.preview
            .clone()
            .unwrap_or_else(|| format!("{}: {}", row.label, row.value))
    } else if focused {
        "Review the selected item before choosing any provider-specific action. No command has run."
            .to_string()
    } else {
        "Press Enter to open the selected explanation. The selected pane shows source, state, and the exact next step without claiming that anything ran.".to_string()
    };
    let lines = vec![
        Line::styled(
            row.label,
            tone_style(row.tone, color).add_modifier(Modifier::BOLD),
        ),
        Line::styled(
            ellipsize(&row.value, inner.width),
            tone_style("info", color),
        ),
        Line::raw(""),
        Line::styled(
            if state.preview_open {
                "Details"
            } else if focused {
                "Review action [U]"
            } else {
                "Next"
            },
            tone_style("muted", color),
        ),
        Line::raw(explanation),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}

fn render_status(
    frame: &mut Frame<'_>,
    area: Rect,
    dashboard: &TuiDashboard,
    state: &TuiState,
    color: bool,
) {
    let text = if state.update_confirmation_active() {
        "confirmation required · type the exact phrase in the dialog"
    } else if state.search_active() {
        "search active · type to filter · Enter accepts · Esc cancels"
    } else if state.show_help {
        "help open · Esc closes"
    } else {
        dashboard.update_action_status.as_str()
    };
    frame.render_widget(
        Paragraph::new(Line::styled(
            ellipsize(&format!("status  {text}"), area.width),
            tone_style("info", color),
        )),
        area,
    );
}

fn render_keys(frame: &mut Frame<'_>, area: Rect, width: u16, color: bool) {
    let keys = if width < 82 {
        "↑↓ move · Tab focus · Enter details · ? help · q quit"
    } else {
        "↑↓/jk move · Tab focus · Enter details · u check · Review action [U] · r refresh · ? help · q quit"
    };
    frame.render_widget(
        Paragraph::new(Line::styled(keys, tone_style("muted", color))),
        area,
    );
}

fn render_help_modal(frame: &mut Frame<'_>, area: Rect, color: bool) {
    let modal = centered_rect(
        area,
        78.min(area.width.saturating_sub(4)),
        12.min(area.height.saturating_sub(4)),
    );
    frame.render_widget(Clear, modal);
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(
                "A calm workspace for safe local operations.",
                tone_style("info", color),
            ),
            Line::raw(""),
            Line::raw("Tab / Shift+Tab  focus workspace, list, selected pane"),
            Line::raw("↑↓ or j/k       move; Home/End jump to boundaries"),
            Line::raw("Enter            open selected details"),
            Line::raw("u / U            review updates / act on one confirmed update"),
            Line::raw("r / m            refresh snapshot / open monitor"),
            Line::raw("/                search software"),
            Line::raw("Esc              close this view; q quits"),
        ])
        .block(panel_block("HELP", "info", color))
        .wrap(Wrap { trim: true }),
        modal,
    );
}

fn render_confirmation_modal(
    frame: &mut Frame<'_>,
    area: Rect,
    dashboard: &TuiDashboard,
    state: &TuiState,
    color: bool,
) {
    let modal = centered_rect(
        area,
        82.min(area.width.saturating_sub(4)),
        10.min(area.height.saturating_sub(4)),
    );
    frame.render_widget(Clear, modal);
    let expected = dashboard
        .pending_update_challenge()
        .map(|challenge| challenge.view.expected_phrase.as_str())
        .unwrap_or("challenge unavailable");
    let lines = vec![
        Line::styled(
            "No update starts until this phrase is entered.",
            tone_style("warn", color),
        ),
        Line::raw(""),
        Line::styled(format!("phrase: {expected}"), tone_style("accent", color)),
        Line::raw(format!("entered: {}", state.update_confirmation_phrase())),
        Line::raw("Enter applies · Esc cancels"),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel_block("CONFIRM ONE ACTION", "warn", color))
            .wrap(Wrap { trim: true }),
        modal,
    );
}

fn render_search_modal(frame: &mut Frame<'_>, area: Rect, state: &TuiState, color: bool) {
    let modal = centered_rect(
        area,
        72.min(area.width.saturating_sub(4)),
        6.min(area.height.saturating_sub(4)),
    );
    frame.render_widget(Clear, modal);
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(
                "Filter the cached software list.",
                tone_style("info", color),
            ),
            Line::raw(format!("query: {}", state.search_query())),
            Line::raw("type · Backspace edit · Enter accepts · Esc cancels"),
        ])
        .block(panel_block("SEARCH", "info", color))
        .wrap(Wrap { trim: true }),
        modal,
    );
}

fn workspace_row(
    row: &TuiRow,
    selected: bool,
    state: &TuiState,
    color: bool,
    width: u16,
) -> Line<'static> {
    let marker = if selected && state.focus_region == TuiFocusRegion::DetailsPanel {
        "▶ "
    } else if selected {
        "· "
    } else {
        "  "
    };
    let label_width = if width >= 60 { 12 } else { 10 };
    let value_width = usize::from(width).saturating_sub(label_width + 4);
    let base = if selected {
        selected_style(color)
    } else {
        Style::default()
    };
    Line::from(vec![
        Span::styled(marker, base),
        Span::styled(
            format!("{:<label_width$}", row.label),
            if selected {
                base
            } else {
                tone_style(row.tone, color)
            },
        ),
        Span::styled(ellipsize(&row.value, value_width as u16), base),
    ])
}

fn workspace_title(section: &TuiSection) -> String {
    match section.title {
        "overview" => "HOME / NEXT STEP".to_string(),
        title => title.to_uppercase(),
    }
}

fn panel_block(title: impl Into<String>, tone: &'static str, color: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(tone_style(tone, color))
        .title(title.into())
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

fn ellipsize(value: &str, width: u16) -> String {
    let width = usize::from(width);
    if width == 0 {
        return String::new();
    }
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= width {
        return value.to_string();
    }
    if width == 1 {
        return "…".to_string();
    }
    let mut result = chars[..width - 1].iter().collect::<String>();
    result.push('…');
    result
}
