use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::tui_dashboard::TuiDashboard;
use crate::tui_ratatui_support::{block, label_line, tone_style};
use crate::tui_theme;

pub(crate) fn render_compact_notice(frame: &mut Frame<'_>, area: Rect, color: bool) {
    let lines = vec![
        Line::styled("runtime.zero", tone_style("accent", color)),
        Line::raw("safe review dashboard"),
        Line::raw("Terminal too small for the full TUI."),
        Line::raw("Use rz0 --no-tui or resize wider/taller."),
        Line::raw("q/Esc exits when interactive."),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(block("COMPACT // SAFE FALLBACK", "info", color)),
        area,
    );
}

pub(crate) fn render_header(
    frame: &mut Frame<'_>,
    area: Rect,
    dashboard: &TuiDashboard,
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
            Span::raw(" read-only foundation"),
        ]),
        Line::from(vec![
            Span::styled("Dossier Navy / Burnished Brass", tone_style("muted", color)),
            Span::raw(" · "),
            Span::styled(
                "no installs · no cleanup · no module execution",
                tone_style("dry_run", color),
            ),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(block("RZ0 // FOUNDATION CONTROL SURFACE", "accent", color)),
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
        Paragraph::new(lines).block(block("FOUNDATION STATE // LIVE", "info", color)),
        area,
    );
}

pub(crate) fn render_footer(frame: &mut Frame<'_>, area: Rect, color: bool) {
    let line = Line::from(vec![
        Span::styled(tui_theme::LABEL_DRY_RUN, tone_style("dry_run", color)),
        Span::raw(" read-only "),
        Span::styled("preview/control surface only", tone_style("accent", color)),
        Span::raw(" · no installs/cleanup/module execution/store writes"),
    ]);
    frame.render_widget(
        Paragraph::new(vec![line]).block(block("SAFETY // LOCKED", "dry_run", color)),
        area,
    );
}

pub(crate) fn preview_only_line(color: bool) -> Line<'static> {
    label_line(
        tui_theme::LABEL_DRY_RUN,
        "PREVIEW ONLY; no command execution from TUI",
        "dry_run",
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
