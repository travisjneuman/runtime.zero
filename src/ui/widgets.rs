use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use super::layout::LayoutPlan;
use super::model::JobState;
use super::model::{ActionDisposition, DetailValue, RecordStatus, Route, UiRecord, ViewState};
use super::state::{FocusRegion, Overlay, UiState};
use super::theme::{Theme, Tone};

pub fn draw_shell(frame: &mut Frame<'_>, state: &UiState, color: bool) {
    let area = frame.area();
    let plan = LayoutPlan::for_area(area);
    let theme = Theme::new(color);
    if plan.tier == super::layout::LayoutTier::VerySmall {
        render_small_notice(frame, area, theme);
        return;
    }
    render_header(frame, plan.header, state, theme);
    render_routes(frame, plan.routes, state, theme);
    render_primary(frame, plan.primary, state, theme);
    render_detail(frame, plan.detail, state, theme);
    render_status(frame, plan.status, state, theme);
    render_keys(frame, plan.keys, state, theme);
    render_overlay(frame, plan.overlay, state, theme);
}

pub fn draw_overview(frame: &mut Frame<'_>, state: &UiState, color: bool) {
    draw_destination(frame, state, color, Route::Overview);
}

pub fn draw_explore(frame: &mut Frame<'_>, state: &UiState, color: bool) {
    draw_destination(frame, state, color, Route::Explore);
}

pub fn draw_review(frame: &mut Frame<'_>, state: &UiState, color: bool) {
    draw_destination(frame, state, color, Route::Review);
}

pub fn draw_activity(frame: &mut Frame<'_>, state: &UiState, color: bool) {
    draw_destination(frame, state, color, Route::Activity);
}

pub fn draw_modules(frame: &mut Frame<'_>, state: &UiState, color: bool) {
    draw_destination(frame, state, color, Route::Modules);
}

fn draw_destination(frame: &mut Frame<'_>, state: &UiState, color: bool, route: Route) {
    debug_assert_eq!(state.route, route);
    draw_shell(frame, state, color);
}

pub fn render_route_screen(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &UiState,
    theme: Theme,
    route: Route,
) {
    let projection = state.model.route(route);
    let title = format!("{} · {}", route.title(), projection.state.label());
    let block = panel(&title, theme.heading());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    render_record_lines(frame, inner, state, theme, &projection.records, route);
}

fn render_header(frame: &mut Frame<'_>, area: Rect, state: &UiState, theme: Theme) {
    let readiness = match &state.model.state {
        ViewState::Loading { .. } => ("loading", Tone::Accent),
        ViewState::Ready { .. } => ("ready", Tone::Safe),
        ViewState::Unavailable { .. } => ("unavailable", Tone::Warn),
        ViewState::Empty { .. } => ("empty", Tone::Info),
        ViewState::Blocked { .. } => ("blocked", Tone::Danger),
        ViewState::Stale { .. } => ("stale", Tone::Warn),
        ViewState::Failed { .. } => ("failed", Tone::Danger),
    };
    let line = Line::from(vec![
        Span::styled("runtime.zero", theme.heading()),
        Span::styled("  /  ", theme.tone(Tone::Muted)),
        Span::styled(state.route.title().to_ascii_uppercase(), theme.heading()),
        Span::styled(
            "                                      ",
            theme.tone(Tone::Muted),
        ),
        Span::styled(readiness.0, theme.tone(readiness.1)),
    ]);
    let subtitle = format!(
        "{}  ·  generation {}  ·  {}",
        route_goal(state.route),
        state.model.generation,
        state.model.status,
    );
    frame.render_widget(
        Paragraph::new(vec![line, Line::from(truncate(&subtitle, area.width))]),
        area,
    );
}

fn route_goal(route: Route) -> &'static str {
    match route {
        Route::Overview => "attention first · choose the next safe step",
        Route::Explore => "evidence index · inspect facts before action",
        Route::Review => "action dossier · verify exact foundation authority",
        Route::Activity => "activity ledger · receipts and recovery stay visible",
        Route::Modules => "module registry · posture without lifecycle control",
    }
}

fn render_routes(frame: &mut Frame<'_>, area: Rect, state: &UiState, theme: Theme) {
    let spans = Route::ALL.into_iter().flat_map(|route| {
        let selected = state.route == route;
        let style = if selected && state.focus == FocusRegion::Routes {
            theme.selected()
        } else if selected {
            theme.heading()
        } else {
            theme.tone(Tone::Muted)
        };
        [Span::styled(
            format!("[{} {}] ", route.number(), route.title()),
            style,
        )]
    });
    frame.render_widget(
        Paragraph::new(Line::from(spans.collect::<Vec<_>>()))
            .block(Block::default().borders(Borders::BOTTOM)),
        area,
    );
}

fn render_primary(frame: &mut Frame<'_>, area: Rect, state: &UiState, theme: Theme) {
    if area.height == 0 {
        return;
    }
    let projection = state.model.route(state.route);
    let block = panel(
        &format!(
            "{}  ·  {} records",
            state.route.title(),
            projection.records.len()
        ),
        if state.focus == FocusRegion::Primary {
            theme.selected()
        } else {
            theme.heading()
        },
    );
    let inner = block.inner(area);
    frame.render_widget(block, area);
    render_record_lines(frame, inner, state, theme, &projection.records, state.route);
}

fn render_record_lines(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &UiState,
    theme: Theme,
    records: &[UiRecord],
    route: Route,
) {
    let query = state.search_query.to_ascii_lowercase();
    let visible = records
        .iter()
        .enumerate()
        .filter(|(_, record)| {
            query.is_empty()
                || record.title.as_str().to_ascii_lowercase().contains(&query)
                || record
                    .summary
                    .as_str()
                    .to_ascii_lowercase()
                    .contains(&query)
                || record
                    .search_terms
                    .0
                    .iter()
                    .any(|term| term.as_str().to_ascii_lowercase().contains(&query))
        })
        .collect::<Vec<_>>();
    let lines = if visible.is_empty() {
        vec![Line::from(Span::styled(
            match &state.model.route(route).state {
                ViewState::Loading { .. } => "[PLAN] loading local evidence",
                ViewState::Unavailable { .. } => "[WARN] evidence unavailable · r refresh",
                ViewState::Empty { .. } => "[INFO] no records are available in this workspace",
                ViewState::Blocked { .. } => "[BLOCKED] evidence is blocked by foundation policy",
                ViewState::Stale { .. } => "[WARN] evidence is stale · r refresh",
                ViewState::Failed { .. } => "[ERROR] evidence failed · r retry explicitly",
                ViewState::Ready { .. } => "[INFO] no records match this view",
            },
            theme.tone(Tone::Muted),
        ))]
    } else {
        visible
            .iter()
            .enumerate()
            .map(|(visible_index, (record_index, record))| {
                let selected = visible_index == state.selected;
                let marker = if selected { "> " } else { "  " };
                let line = format!(
                    "{marker}{:<9} {}",
                    record.status.label(),
                    truncate(record.title.as_str(), area.width.saturating_sub(13)),
                );
                let style = if selected && state.focus == FocusRegion::Primary {
                    theme.selected()
                } else if selected {
                    Style::default().add_modifier(ratatui::style::Modifier::BOLD)
                } else {
                    theme.status(record.status)
                };
                let _ = record_index;
                Line::from(Span::styled(line, style))
            })
            .collect::<Vec<_>>()
    };
    frame.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: true }),
        area,
    );
}

fn render_detail(frame: &mut Frame<'_>, area: Rect, state: &UiState, theme: Theme) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let title = if state.focus == FocusRegion::Detail {
        "Selected detail · focused"
    } else {
        "Selected detail"
    };
    let block = panel(title, theme.heading());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let Some(record) = state.selected_record() else {
        frame.render_widget(
            Paragraph::new("Select an evidence record to inspect it."),
            inner,
        );
        return;
    };
    let mut lines = vec![
        Line::from(Span::styled(
            format!("{}  {}", record.status.label(), record.title),
            theme
                .status(record.status)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )),
        Line::from(Span::styled(
            truncate(record.summary.as_str(), inner.width),
            theme.tone(Tone::Info),
        )),
        Line::raw(""),
    ];
    for section in &record.details {
        lines.push(Line::from(Span::styled(
            section.title.as_str(),
            theme.tone(Tone::Muted),
        )));
        for field in &section.fields {
            lines.push(Line::from(format!(
                "{}: {}",
                field.label,
                detail_value(&field.value)
            )));
        }
    }
    if let Some(action) = record.action_refs.first() {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!("Review action: {}", action.disposition.label()),
            theme.status(if action.disposition == ActionDisposition::Blocked {
                RecordStatus::Blocked
            } else {
                RecordStatus::Plan
            }),
        )));
        lines.push(Line::raw("Enter opens the read-only authority boundary."));
    } else if let Some(boundary) = &record.review_boundary {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!("Review boundary: {}", boundary.disposition.label()),
            theme.tone(Tone::Info),
        )));
        lines.push(Line::raw(boundary.message.as_str()));
        lines.push(Line::raw("U opens review; no action is executed."));
    } else {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "No action has run. Evidence remains read-only.",
            theme.tone(Tone::Muted),
        )));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}

fn render_status(frame: &mut Frame<'_>, area: Rect, state: &UiState, theme: Theme) {
    let job = match &state.job {
        JobState::Idle => "idle",
        JobState::Running { phase, .. } => phase.as_str(),
        JobState::Succeeded { .. } => "succeeded · receipt verified",
        JobState::Cancelled { .. } => "cancelled · not rollback",
        JobState::Recovery { .. } => "recovery review required",
        JobState::Failed { .. } => "failed · foundation rejected or could not complete",
    };
    let status = format!("{} · job {job}", state.model.status);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("status  ", theme.tone(Tone::Muted)),
            Span::styled(
                truncate(&status, area.width.saturating_sub(8)),
                theme.tone(Tone::Info),
            ),
        ])),
        area,
    );
}

fn render_keys(frame: &mut Frame<'_>, area: Rect, state: &UiState, theme: Theme) {
    let text = if state.search_active {
        "search  type to filter · Enter accept · Esc cancel"
    } else if state.confirmation.is_some() {
        "confirmation  type exact foundation phrase · Enter submit · Esc cancel"
    } else {
        "↑↓/jk move · Tab focus · Enter detail/review · c confirm · / search · r refresh · ? help · q quit"
    };
    frame.render_widget(
        Paragraph::new(Line::styled(text, theme.tone(Tone::Muted))),
        area,
    );
}

fn render_overlay(frame: &mut Frame<'_>, area: Rect, state: &UiState, theme: Theme) {
    let overlay = match &state.overlay {
        Overlay::None => return,
        overlay => overlay,
    };
    frame.render_widget(Clear, area);
    let (title, lines): (&str, Vec<String>) = match overlay {
        Overlay::Help => (
            "Help",
            vec![
                "Tab / Shift+Tab  focus routes, records, detail, footer".to_string(),
                "↑↓ or j/k       move; Home/End jump to boundaries".to_string(),
                "Enter            open detail or read-only action review".to_string(),
                "/                search current typed records".to_string(),
                "u                explicit provider review; no automatic retry".to_string(),
                "r                refresh local evidence; no automatic retry".to_string(),
                "Esc              close overlay; q quits safely".to_string(),
            ],
        ),
        Overlay::Search => (
            "Search",
            vec![
                "Local, read-only filter over the current typed record set.".to_string(),
                format!("query: {}", state.search_query),
                "type · Backspace edit · Enter accepts · Esc cancels".to_string(),
            ],
        ),
        Overlay::Detail => (
            "Evidence detail",
            vec![
                "This view is read-only.".to_string(),
                "Esc returns to the selected record.".to_string(),
            ],
        ),
        Overlay::ActionReview(action_id) => (
            "Read-only action review",
            action_review_lines(state, action_id),
        ),
        Overlay::Confirmation(action_id) => (
            "Foundation confirmation",
            confirmation_lines(state, action_id),
        ),
        Overlay::Recovery(transaction) => (
            "Recovery evidence",
            vec![
                "Recovery review is read-only.".to_string(),
                format!("transaction: {transaction}"),
                "Do not rerun, repair, rollback, or complete from the UI.".to_string(),
            ],
        ),
        Overlay::None => unreachable!(),
    };
    let content = lines.into_iter().map(Line::from).collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(content)
            .block(panel(title, theme.heading()))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn confirmation_lines(state: &UiState, action_id: &super::model::BoundedId) -> Vec<String> {
    let Some(prompt) = state.confirmation.as_ref() else {
        return vec![
            format!("reference: {action_id}"),
            "confirmation challenge is unavailable".to_string(),
            "Esc cancels without execution.".to_string(),
        ];
    };
    vec![
        "The foundation owns confirmation validation and execution.".to_string(),
        format!("action: {}", prompt.action_id),
        format!("plan: {}", prompt.plan_id),
        format!("target: {}", prompt.target),
        format!("risk: {}", prompt.risk),
        format!("plan sha256: {}", prompt.plan_sha256),
        format!(
            "rollback: {}",
            if prompt.rollback_available {
                "available"
            } else {
                "not established"
            }
        ),
        format!(
            "manual recovery acknowledgement: {}",
            if prompt.manual_recovery_acknowledged {
                "recorded"
            } else {
                "required by foundation"
            }
        ),
        format!("type exactly: {}", prompt.expected_phrase),
        format!("input: {}", state.confirmation_input),
        format!("expires: unix {}", prompt.expires_unix_seconds),
        "Enter submits to foundation validation · Esc cancels".to_string(),
    ]
}

fn action_review_lines(state: &UiState, action_id: &super::model::BoundedId) -> Vec<String> {
    let Some(record) = state.selected_record() else {
        return vec![
            "No action has run.".to_string(),
            format!("reference: {action_id}"),
            "foundation review evidence is unavailable".to_string(),
            "Esc closes this review.".to_string(),
        ];
    };
    if let Some(action) = record
        .action_refs
        .iter()
        .find(|action| action.action_id == *action_id)
    {
        return vec![
            "No action has run.".to_string(),
            format!("reference: {}", action.action_id),
            format!("operation: {}", action.review.operation),
            format!("target: {}", action.review.target),
            format!("authority: {}", action.review.authority),
            format!("plan: {}", action.review.plan_id),
            format!("plan sha256: {}", action.review.plan_sha256),
            format!("write set sha256: {}", action.review.write_set_sha256),
            format!("risk: {}", action.review.risk),
            format!(
                "confirmation: {}",
                if action.review.requires_confirmation {
                    "required"
                } else {
                    "not required"
                }
            ),
            format!(
                "elevation: {} · network: {}",
                if action.review.requires_elevation {
                    "required"
                } else {
                    "not required"
                },
                if action.review.network_required {
                    "required"
                } else {
                    "not required"
                },
            ),
            format!(
                "capabilities: {}",
                if action.review.capabilities.is_empty() {
                    "none".to_string()
                } else {
                    action
                        .review
                        .capabilities
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            ),
            format!("rollback: {}", action.review.rollback),
            "recovery: foundation transaction and recovery evidence".to_string(),
            format!(
                "executed: {}",
                if action.review.executed { "yes" } else { "no" }
            ),
            "Esc closes this review.".to_string(),
        ];
    }
    if let Some(boundary) = record
        .review_boundary
        .as_ref()
        .filter(|boundary| boundary.reference_id == *action_id)
    {
        return vec![
            "No action has run.".to_string(),
            format!("reference: {}", boundary.reference_id),
            format!("disposition: {}", boundary.disposition.label()),
            "confirmation: foundation-owned after plan validation".to_string(),
            "recovery: foundation transaction and recovery evidence".to_string(),
            boundary.message.as_str().to_string(),
            "Esc closes this review.".to_string(),
        ];
    }
    vec![
        "No action has run.".to_string(),
        format!("reference: {action_id}"),
        "foundation review evidence is unavailable".to_string(),
        "Esc closes this review.".to_string(),
    ]
}

fn render_small_notice(frame: &mut Frame<'_>, area: Rect, theme: Theme) {
    let lines = vec![
        Line::from(Span::styled("runtime.zero", theme.heading())),
        Line::raw("Terminal too small for the operator console."),
        Line::raw("Resize to at least 50x12 or use:"),
        Line::from(Span::styled("rz0 --no-tui", theme.tone(Tone::Info))),
        Line::raw("q / Esc  exit"),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(ratatui::layout::Alignment::Center)
            .block(panel("Terminal too small", theme.tone(Tone::Info)))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn panel(title: &str, style: Style) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(style)
        .title(title.to_string())
}

fn detail_value(value: &DetailValue) -> String {
    match value {
        DetailValue::Text(value)
        | DetailValue::Version(value)
        | DetailValue::Digest(value)
        | DetailValue::Timestamp(value) => value.to_string(),
        DetailValue::Count(value) => value.to_string(),
        DetailValue::Status(value) => value.label().to_string(),
        DetailValue::Reference(value) => value.to_string(),
    }
}

fn truncate(value: &str, width: u16) -> String {
    let width = usize::from(width);
    if value.chars().count() <= width {
        return value.to_string();
    }
    if width <= 1 {
        return "…".to_string();
    }
    let mut output = value.chars().take(width - 1).collect::<String>();
    output.push('…');
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::testkit::{fixture_model, frame_text};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn dossier_shell_has_required_first_screen_labels_without_color() {
        let model = fixture_model();
        let state = UiState::new(model);
        let mut terminal = Terminal::new(TestBackend::new(118, 30)).expect("test terminal");
        terminal
            .draw(|frame| draw_shell(frame, &state, false))
            .expect("draw");
        let text = frame_text(terminal.backend().buffer());
        for label in [
            "runtime.zero",
            "Overview",
            "Explore",
            "Review",
            "Activity",
            "Modules",
        ] {
            assert!(text.contains(label), "missing {label} in {text}");
        }
        assert!(
            text.contains("No action has run")
                || text.contains("Review action")
                || text.contains("Review boundary")
        );
    }
}
