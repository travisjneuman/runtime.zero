use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::tui_dashboard::TuiDashboard;
use crate::tui_layout::TuiLayoutTier;
use crate::tui_ratatui_support::{
    block, label_line, selected_index, selected_row_index, selected_section, tone_style,
};
use crate::tui_state::{TuiFocusRegion, TuiState};
use crate::tui_theme;

pub(crate) fn render_compact_notice(
    frame: &mut Frame<'_>,
    area: Rect,
    tier: TuiLayoutTier,
    color: bool,
) {
    let lines = vec![
        Line::styled("runtime.zero", tone_style("accent", color)),
        Line::raw("installed software and available actions"),
        Line::raw(format!(
            "layout: {} · min {}",
            tier.name(),
            tier.minimum_size()
        )),
        Line::raw("Terminal too small for the full TUI frame."),
        Line::raw("Use rz0 --no-tui or resize wider/taller."),
        Line::raw("q/Esc exits when interactive."),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(block("COMPACT // DASHBOARD", "info", color)),
        area,
    );
}

pub(crate) fn render_compact_dashboard(
    frame: &mut Frame<'_>,
    area: Rect,
    dashboard: &TuiDashboard,
    state: &TuiState,
    tier: TuiLayoutTier,
    color: bool,
) {
    let section = selected_section(dashboard, state);
    let selected_row = selected_row_index(section, state);
    let mut lines = vec![
        Line::from(vec![
            Span::styled("runtime.zero rz0", tone_style("accent", color)),
            Span::raw("   "),
            Span::styled(tui_theme::LABEL_OK, tone_style("safe", color)),
            Span::raw(" ready"),
        ]),
        Line::raw(format!(
            "layout: {} · section {} / {}",
            tier.name(),
            selected_index(dashboard, state) + 1,
            dashboard.sections.len()
        )),
        Line::raw(format!("section {} · {}", section.code, section.title)),
        Line::raw(section.summary),
        Line::raw(format!(
            "store {:?} · registry {:?} · modules {}",
            dashboard.store_init_status, dashboard.registry_state, dashboard.installed_module_count
        )),
    ];
    if let Some(row) = section.rows.get(selected_row) {
        lines.push(Line::raw(format!(
            "item {}/{}: {} {}",
            selected_row + 1,
            section.rows.len(),
            row.label,
            row.value
        )));
        if state.preview_open && state.focus_region == TuiFocusRegion::DetailsPanel {
            lines.push(details_line(color));
            lines.push(Line::raw(
                row.preview
                    .clone()
                    .unwrap_or_else(|| format!("{}: {}", row.label, row.value)),
            ));
        }
    } else {
        lines.push(Line::raw("item: no detail rows reported"));
    }
    if !(state.preview_open && state.focus_region == TuiFocusRegion::DetailsPanel) {
        lines.push(details_line(color));
    }
    lines.push(Line::raw(
        "q exits · m monitor · u updates · / search · f filter · s sort · r refresh",
    ));
    frame.render_widget(
        Paragraph::new(lines).block(block("COMPACT // DASHBOARD", "info", color)),
        area,
    );
}

pub(crate) fn render_header(
    frame: &mut Frame<'_>,
    area: Rect,
    dashboard: &TuiDashboard,
    tier: TuiLayoutTier,
    color: bool,
) {
    let lines = vec![
        Line::from(vec![
            Span::styled("runtime.zero", tone_style("accent", color)),
            Span::raw("  "),
            Span::styled("rz0", tone_style("info", color)),
            Span::raw(format!("  v{}", dashboard.version)),
            Span::raw("   "),
            Span::styled(tui_theme::LABEL_OK, tone_style("safe", color)),
            Span::raw(" live local inventory"),
        ]),
        Line::from(vec![
            Span::styled("local inventory", tone_style("muted", color)),
            Span::raw(" · "),
            Span::raw(tier.name()),
            Span::raw(" · "),
            Span::styled(
                "inventory live · updates available from the actions",
                tone_style("info", color),
            ),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(block("RZ0 // INSTALLED SOFTWARE", "accent", color)),
        area,
    );
}

pub(crate) fn render_state_cards(
    frame: &mut Frame<'_>,
    area: Rect,
    dashboard: &TuiDashboard,
    color: bool,
) {
    let lines = vec![
        status_pair_line(
            "store",
            &format!("{:?}", dashboard.store_init_status).to_lowercase(),
            "registry",
            &format!("{:?}", dashboard.registry_state).to_lowercase(),
            color,
        ),
        status_pair_line(
            "receipts",
            &format!("{:?}", dashboard.receipt_state).to_lowercase(),
            "modules",
            &format!("{} installed", dashboard.installed_module_count),
            color,
        ),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(block("STATUS", "info", color)),
        area,
    );
}

pub(crate) fn render_footer(frame: &mut Frame<'_>, area: Rect, color: bool) {
    let line = Line::from(vec![
        Span::styled(tui_theme::LABEL_OK, tone_style("safe", color)),
        Span::raw(" "),
        Span::styled("installed software", tone_style("accent", color)),
        Span::raw(" · Enter details · m monitor · u checks updates"),
    ]);
    frame.render_widget(
        Paragraph::new(vec![line]).block(block("ACTIONS", "info", color)),
        area,
    );
}

pub(crate) fn details_line(color: bool) -> Line<'static> {
    label_line(
        tui_theme::LABEL_INFO,
        "selected item details",
        "info",
        color,
    )
}

fn status_pair_line(
    left_label: &'static str,
    left_value: &str,
    right_label: &'static str,
    right_value: &str,
    color: bool,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{left_label:<9}"), tone_style("info", color)),
        Span::raw(left_value.to_string()),
        Span::raw("   "),
        Span::styled(format!("{right_label:<9}"), tone_style("info", color)),
        Span::raw(right_value.to_string()),
    ])
}
