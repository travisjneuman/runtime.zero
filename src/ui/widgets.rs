use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use super::layout::{LayoutPlan, LayoutTier};
use super::model::{ActionDisposition, DetailValue, JobState, UiRecord, ViewState};
use super::state::{FocusRegion, Overlay, UiPage, UiState};
use super::theme::{Theme, Tone};

pub fn draw_shell(frame: &mut Frame<'_>, state: &UiState, color: bool) {
    let area = frame.area();
    let plan = LayoutPlan::for_area(area);
    let theme = Theme::new(color);
    if plan.tier == LayoutTier::VerySmall {
        render_small_notice(frame, area, theme);
        return;
    }

    render_header(frame, plan.header, state, theme);
    render_context(frame, plan.context, state, theme);
    match state.page {
        UiPage::Home | UiPage::Inventory => {
            render_queue(frame, plan.primary, state, theme);
            render_selected_detail(frame, plan.detail, state, theme);
        }
        UiPage::Evidence => {
            render_evidence(frame, plan.primary, state, theme);
            render_next_step(frame, plan.detail, state, theme);
        }
        UiPage::Review => {
            render_plan(frame, plan.primary, state, theme);
            render_review_authority(frame, plan.detail, state, theme);
        }
        UiPage::Confirmation => {
            render_confirmation(frame, plan.primary, state, theme);
            render_confirmation_boundary(frame, plan.detail, state, theme);
        }
        UiPage::Activity => {
            render_activity(frame, plan.primary, state, theme);
            render_selected_detail(frame, plan.detail, state, theme);
        }
    }
    render_status(frame, plan.status, state, theme);
    render_keys(frame, plan.keys, state, theme);
    render_overlay(frame, plan.overlay, state, theme);
}

fn render_header(frame: &mut Frame<'_>, area: Rect, state: &UiState, theme: Theme) {
    let (label, tone) = view_state_label(&state.model.state);
    let title = match state.page {
        UiPage::Home => "HOME",
        UiPage::Inventory => "INVENTORY",
        UiPage::Evidence => "EVIDENCE",
        UiPage::Review => "PLAN REVIEW",
        UiPage::Confirmation => "CONFIRMATION",
        UiPage::Activity => "ACTIVITY",
    };
    let line = Line::from(vec![
        Span::styled("runtime.zero", theme.heading()),
        Span::styled("  /  ", theme.tone(Tone::Muted)),
        Span::styled(title, theme.heading()),
        Span::styled("  ", theme.tone(Tone::Muted)),
        Span::styled(label, theme.tone(tone)),
    ]);
    let subtitle = match state.page {
        UiPage::Home => "what needs attention · the safest next action",
        UiPage::Inventory => "inspect current foundation evidence",
        UiPage::Evidence => "verify source, scope, freshness, and disposition",
        UiPage::Review => "read the exact foundation plan before deciding",
        UiPage::Confirmation => "a dedicated foundation challenge is required",
        UiPage::Activity => "progress, cancellation, receipt, and recovery evidence",
    };
    frame.render_widget(
        Paragraph::new(vec![line, Line::from(truncate(subtitle, area.width))]),
        area,
    );
}

fn render_context(frame: &mut Frame<'_>, area: Rect, state: &UiState, theme: Theme) {
    let context = match state.page {
        UiPage::Home => "next safe action  ·  attention is explicit".to_string(),
        UiPage::Inventory => "inventory  ·  foundation evidence and module content".to_string(),
        UiPage::Evidence => "selected evidence  ·  read-only".to_string(),
        UiPage::Review => "exact plan  ·  foundation authority".to_string(),
        UiPage::Confirmation => "confirmation  ·  no shortcut".to_string(),
        UiPage::Activity => "activity  ·  current outcome is explicit".to_string(),
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  ", theme.tone(Tone::Muted)),
            Span::styled(
                truncate(&context, area.width.saturating_sub(2)),
                theme.tone(Tone::Info),
            ),
        ])),
        area,
    );
}

fn render_queue(frame: &mut Frame<'_>, area: Rect, state: &UiState, theme: Theme) {
    let title = if state.page == UiPage::Home {
        "Next safe actions"
    } else {
        "Current evidence"
    };
    let block = panel(
        title,
        if state.focus == FocusRegion::Queue {
            theme.selected()
        } else {
            theme.heading()
        },
    );
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let locators = state.current_records();
    if locators.is_empty() {
        render_state_message(frame, inner, state, state.page == UiPage::Home, theme);
        return;
    }
    let row_height = 2usize;
    let max_rows = usize::from(inner.height).div_ceil(row_height);
    let lines = locators
        .iter()
        .enumerate()
        .take(max_rows)
        .flat_map(|(visible_index, locator)| {
            let selected = visible_index == state.selected;
            let record = state
                .model
                .route(locator.route)
                .records
                .get(locator.index)
                .expect("state locator is valid");
            let marker = if selected { ">" } else { " " };
            let status = if record
                .action_refs
                .iter()
                .any(|action| action.disposition == ActionDisposition::Reviewable)
            {
                "REVIEW"
            } else {
                record.status.label().trim_matches(&['[', ']'][..])
            };
            let title = format!("{marker} {status}  {}", record.title);
            let summary = format!("  {}", record.summary);
            let style = if selected && state.focus == FocusRegion::Queue {
                theme.selected()
            } else if selected {
                theme.heading().add_modifier(Modifier::BOLD)
            } else {
                theme.status(record.status)
            };
            [
                Line::from(Span::styled(truncate(&title, inner.width), style)),
                Line::from(Span::styled(
                    truncate(&summary, inner.width),
                    theme.tone(Tone::Muted),
                )),
            ]
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

fn render_selected_detail(frame: &mut Frame<'_>, area: Rect, state: &UiState, theme: Theme) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let block = panel(
        "Selected evidence",
        if state.focus == FocusRegion::Detail {
            theme.selected()
        } else {
            theme.heading()
        },
    );
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let Some(record) = state.selected_record() else {
        frame.render_widget(Paragraph::new("Nothing is selected."), inner);
        return;
    };
    let mut lines = vec![
        Line::from(Span::styled(
            format!("{}  {}", record.status.label(), record.title),
            theme.status(record.status).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            truncate(record.summary.as_str(), inner.width),
            theme.tone(Tone::Info),
        )),
        Line::raw(""),
    ];
    push_evidence_fields(&mut lines, record, theme);
    if has_action_plan(record) {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "next: Enter inspect the exact plan",
            theme.tone(Tone::Accent),
        )));
    } else if has_review(record) {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "next: u request explicit provider review",
            theme.tone(Tone::Accent),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "next: i inspect the full inventory",
            theme.tone(Tone::Muted),
        )));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}

fn render_evidence(frame: &mut Frame<'_>, area: Rect, state: &UiState, theme: Theme) {
    let block = panel("Evidence dossier", theme.heading());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let Some(record) = state.selected_record() else {
        frame.render_widget(
            Paragraph::new("The selected evidence is unavailable."),
            inner,
        );
        return;
    };
    let mut lines = vec![
        Line::from(Span::styled(
            format!("{}  {}", record.status.label(), record.title),
            theme.status(record.status).add_modifier(Modifier::BOLD),
        )),
        Line::from(record.summary.as_str().to_string()),
        Line::raw(""),
    ];
    for section in &record.details {
        lines.push(Line::from(Span::styled(
            section.title.as_str(),
            theme.tone(Tone::Accent),
        )));
        for field in &section.fields {
            lines.push(Line::from(format!(
                "{}: {}",
                field.label,
                detail_value(&field.value)
            )));
        }
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        if has_action_plan(record) {
            "Enter opens the exact plan review. No action has run."
        } else if has_review(record) {
            "Enter opens the read-only boundary. Use u for provider review."
        } else {
            "This evidence is read-only. No action is available."
        },
        theme.tone(Tone::Info),
    )));
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}

fn render_next_step(frame: &mut Frame<'_>, area: Rect, state: &UiState, theme: Theme) {
    let block = panel("Safest next step", theme.heading());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let Some(record) = state.selected_record() else {
        frame.render_widget(Paragraph::new("Return to Home and select an item."), inner);
        return;
    };
    let lines = if has_action_plan(record) {
        vec![
            Line::from(Span::styled("1  inspect evidence", theme.tone(Tone::Safe))),
            Line::from(Span::styled("2  read exact plan", theme.tone(Tone::Accent))),
            Line::from("3  confirm only after review"),
        ]
    } else if has_review(record) {
        vec![
            Line::from(Span::styled("1  inspect boundary", theme.tone(Tone::Safe))),
            Line::from(Span::styled(
                "2  u request provider review",
                theme.tone(Tone::Accent),
            )),
            Line::from("3  return here when an exact plan exists"),
        ]
    } else {
        vec![
            Line::from(Span::styled("observe only", theme.tone(Tone::Info))),
            Line::from("no foundation action is available for this item"),
        ]
    };
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}

fn render_plan(frame: &mut Frame<'_>, area: Rect, state: &UiState, theme: Theme) {
    let block = panel("Exact plan · not executed", theme.heading());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let Some(record) = state.selected_record() else {
        frame.render_widget(Paragraph::new("No plan is selected."), inner);
        return;
    };
    let lines = review_lines(record, theme);
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}

fn render_review_authority(frame: &mut Frame<'_>, area: Rect, state: &UiState, theme: Theme) {
    let block = panel("Authority boundary", theme.heading());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let Some(record) = state.selected_record() else {
        frame.render_widget(Paragraph::new("No foundation authority is present."), inner);
        return;
    };
    let mut lines = vec![
        Line::from(Span::styled("foundation owns", theme.tone(Tone::Accent))),
        Line::from("confirmation, execution, transaction, receipt, verification"),
        Line::raw(""),
    ];
    if let Some(action) = record.action_refs.first() {
        lines.extend([
            Line::from(format!("action: {}", action.action_id)),
            Line::from(format!("disposition: {}", action.disposition.label())),
            Line::from(format!(
                "confirmation: {}",
                if action.review.requires_confirmation {
                    "required"
                } else {
                    "not required"
                }
            )),
            Line::from("execution: not run"),
            Line::raw(""),
            Line::from(Span::styled(
                if action.disposition == ActionDisposition::Reviewable {
                    "c prepare the foundation challenge"
                } else {
                    "blocked: no confirmation path is available"
                },
                theme.tone(if action.disposition == ActionDisposition::Reviewable {
                    Tone::Accent
                } else {
                    Tone::Warn
                }),
            )),
        ]);
    } else if let Some(boundary) = &record.review_boundary {
        lines.extend([
            Line::from(format!("disposition: {}", boundary.disposition.label())),
            Line::from(boundary.message.as_str().to_string()),
            Line::from("provider review may be required before a plan exists"),
        ]);
    } else {
        lines.push(Line::from("no action boundary exists for this evidence"));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}

fn render_confirmation(frame: &mut Frame<'_>, area: Rect, state: &UiState, theme: Theme) {
    let block = panel("Dedicated confirmation state", theme.tone(Tone::Warn));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let Some(prompt) = state.confirmation.as_ref() else {
        frame.render_widget(
            Paragraph::new("The foundation challenge is unavailable."),
            inner,
        );
        return;
    };
    let lines = vec![
        Line::from(Span::styled(
            "Nothing will run until the foundation accepts this exact phrase.",
            theme.tone(Tone::Warn),
        )),
        Line::raw(""),
        Line::from(format!("target: {}", prompt.target)),
        Line::from(format!("plan: {}", prompt.plan_id)),
        Line::from(format!("plan sha256: {}", prompt.plan_sha256)),
        Line::from(format!("risk: {}", prompt.risk)),
        Line::from(format!(
            "rollback: {}",
            if prompt.rollback_available {
                "available"
            } else {
                "not established"
            }
        )),
        Line::raw(""),
        Line::from(Span::styled(
            format!("type exactly: {}", prompt.expected_phrase),
            theme.tone(Tone::Accent),
        )),
        Line::from(format!("input: {}", state.confirmation_input)),
        Line::raw(""),
        Line::from("Enter submits to foundation validation · Esc cancels"),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}

fn render_confirmation_boundary(frame: &mut Frame<'_>, area: Rect, state: &UiState, theme: Theme) {
    let block = panel("Before submit", theme.heading());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let Some(prompt) = state.confirmation.as_ref() else {
        frame.render_widget(Paragraph::new("Esc returns without execution."), inner);
        return;
    };
    let lines = vec![
        Line::from("Review the target and digest above."),
        Line::from(format!(
            "manual recovery acknowledgement: {}",
            if prompt.manual_recovery_acknowledged {
                "recorded"
            } else {
                "required by foundation"
            }
        )),
        Line::raw(""),
        Line::from(Span::styled("Esc cancels safely.", theme.tone(Tone::Info))),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}

fn render_activity(frame: &mut Frame<'_>, area: Rect, state: &UiState, theme: Theme) {
    let block = panel("Activity · outcome", theme.heading());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let (headline, lines, tone) = match &state.job {
        JobState::Idle => (
            "idle · no action is running",
            vec!["Provider review and foundation actions are explicit.".to_string()],
            Tone::Info,
        ),
        JobState::Running { job_id, phase } => (
            "running · progress is visible",
            vec![
                format!("job: {job_id}"),
                format!("phase: {phase}"),
                "Esc cancels.".to_string(),
            ],
            Tone::Accent,
        ),
        JobState::Succeeded {
            receipt,
            verification,
        } => (
            "verified · fresh evidence recorded",
            vec![
                format!("receipt: {receipt}"),
                format!("verification: {verification}"),
                "Refresh before another decision.".to_string(),
            ],
            Tone::Safe,
        ),
        JobState::Cancelled { job_id, reason } => (
            "cancelled · no rollback was implied",
            vec![
                format!("job: {job_id}"),
                format!("reason: {reason}"),
                "Return to Home or refresh explicitly.".to_string(),
            ],
            Tone::Warn,
        ),
        JobState::Recovery {
            transaction,
            decision,
        } => (
            "recovery-required · read-only review",
            vec![
                format!("transaction: {transaction}"),
                format!("decision: {decision}"),
                "Do not rerun, repair, or rollback here.".to_string(),
            ],
            Tone::Warn,
        ),
        JobState::Failed { job_id, reason } => (
            "failed · foundation outcome is visible",
            vec![
                format!("job: {job_id}"),
                format!("reason: {reason}"),
                "Review evidence before any new request.".to_string(),
            ],
            Tone::Danger,
        ),
    };
    let mut content = vec![Line::from(Span::styled(headline, theme.tone(tone)))];
    content.extend(lines.into_iter().map(Line::from));
    content.push(Line::raw(""));
    content.push(Line::from(Span::styled(
        format!("state: {}", state.view_state().label()),
        theme.tone(tone),
    )));
    frame.render_widget(Paragraph::new(content).wrap(Wrap { trim: true }), inner);
}

fn render_status(frame: &mut Frame<'_>, area: Rect, state: &UiState, theme: Theme) {
    let job = match &state.job {
        JobState::Idle => "idle",
        JobState::Running { phase, .. } => phase.as_str(),
        JobState::Succeeded { .. } => "verified",
        JobState::Cancelled { .. } => "cancelled",
        JobState::Recovery { .. } => "recovery-required",
        JobState::Failed { .. } => "failed",
    };
    let status = format!("{}  ·  job {job}", state.model.status);
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
    } else {
        match state.page {
            UiPage::Home => {
                "↑↓ select · Enter inspect · i inventory · a activity · u review providers · r refresh · ? help · q quit"
            }
            UiPage::Inventory => {
                "↑↓ select · Enter inspect · / search · h home · a activity · Esc back"
            }
            UiPage::Evidence => "Enter plan review · Tab detail/controls · Esc back · ? help",
            UiPage::Review => "c prepare exact foundation confirmation · Esc back · ? help",
            UiPage::Confirmation => "type exact phrase · Enter submit · Esc cancel",
            UiPage::Activity => "Esc back · r refresh evidence · ? help · q quit",
        }
    };
    frame.render_widget(
        Paragraph::new(Line::styled(text, theme.tone(Tone::Muted))),
        area,
    );
}

fn render_overlay(frame: &mut Frame<'_>, area: Rect, state: &UiState, theme: Theme) {
    let overlay = match state.overlay {
        Overlay::None => return,
        ref overlay => overlay,
    };
    frame.render_widget(Clear, area);
    let (title, lines) = match overlay {
        Overlay::Help => (
            "Controls",
            vec![
                "Home is the attention queue; modules appear as evidence content.".to_string(),
                "Enter: inspect evidence, then open the exact plan.".to_string(),
                "c: only inside Plan Review; prepare the foundation challenge.".to_string(),
                "u: explicit provider review; no automatic retry or writes.".to_string(),
                "i: inventory · a: activity · h: home · /: local search.".to_string(),
                "Tab: queue, detail, controls · Esc: back/cancel · q: CLI escape.".to_string(),
            ],
        ),
        Overlay::Search => (
            "Search current evidence",
            vec![
                "Local filtering only; no provider request is made.".to_string(),
                format!("query: {}", state.search_query),
                "type · Backspace edit · Enter accept · Esc cancel".to_string(),
            ],
        ),
        Overlay::None => unreachable!(),
    };
    frame.render_widget(
        Paragraph::new(lines.into_iter().map(Line::from).collect::<Vec<_>>())
            .block(panel(title, theme.heading()))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_state_message(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &UiState,
    home: bool,
    theme: Theme,
) {
    let message = match state.view_state() {
        ViewState::Loading { .. } => "loading local evidence · the first frame is safe to inspect",
        ViewState::Refreshing { .. } => {
            "refreshing local evidence · the previous decision is not reused"
        }
        ViewState::Ready { .. } if home => "ready · no attention item was reported",
        ViewState::Ready { .. } => "ready · no evidence matches this search",
        ViewState::Unavailable { reason, .. } => {
            return render_message(
                frame,
                area,
                &format!("unavailable · {reason}"),
                theme,
                Tone::Warn,
            );
        }
        ViewState::Empty { .. } => "empty · collection succeeded and found nothing",
        ViewState::Blocked { reason, .. } => {
            return render_message(
                frame,
                area,
                &format!("blocked · {reason}"),
                theme,
                Tone::Warn,
            );
        }
        ViewState::Stale { reason, .. } => {
            return render_message(
                frame,
                area,
                &format!("stale · {reason} · r refresh explicitly"),
                theme,
                Tone::Warn,
            );
        }
        ViewState::Cancelled { reason, .. } => {
            return render_message(
                frame,
                area,
                &format!("cancelled · {reason}"),
                theme,
                Tone::Warn,
            );
        }
        ViewState::Verified { .. } => "verified · fresh post-action evidence is recorded",
        ViewState::RecoveryRequired { reason, .. } => {
            return render_message(
                frame,
                area,
                &format!("recovery-required · {reason}"),
                theme,
                Tone::Warn,
            );
        }
        ViewState::Failed { reason, .. } => {
            return render_message(
                frame,
                area,
                &format!("failed · {reason}"),
                theme,
                Tone::Danger,
            );
        }
    };
    render_message(frame, area, message, theme, Tone::Info);
}

fn render_message(frame: &mut Frame<'_>, area: Rect, message: &str, theme: Theme, tone: Tone) {
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(message, theme.tone(tone))))
            .wrap(Wrap { trim: true }),
        area,
    );
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

fn push_evidence_fields(lines: &mut Vec<Line<'static>>, record: &UiRecord, theme: Theme) {
    for evidence in &record.evidence {
        lines.push(Line::from(format!(
            "source: {:?} · freshness: {:?}",
            evidence.source, evidence.freshness
        )));
    }
    if let Some(boundary) = &record.review_boundary {
        lines.push(Line::from(Span::styled(
            format!("disposition: {}", boundary.disposition.label()),
            theme.tone(Tone::Accent),
        )));
    }
}

fn review_lines(record: &UiRecord, theme: Theme) -> Vec<Line<'static>> {
    let Some(action) = record.action_refs.first() else {
        return vec![
            Line::from(Span::styled(
                "No exact action plan is available.",
                theme.tone(Tone::Warn),
            )),
            Line::from("This is an evidence boundary, not permission to act."),
        ];
    };
    let review = &action.review;
    vec![
        Line::from(Span::styled(
            "not executed · review only",
            theme.tone(Tone::Warn),
        )),
        Line::from(format!("operation: {}", review.operation)),
        Line::from(format!("target: {}", review.target)),
        Line::from(format!("authority: {}", review.authority)),
        Line::from(format!("plan: {}", review.plan_id)),
        Line::from(format!("plan sha256: {}", review.plan_sha256)),
        Line::from(format!("write set sha256: {}", review.write_set_sha256)),
        Line::from(format!("risk: {}", review.risk)),
        Line::from(format!(
            "network: {} · elevation: {}",
            required(review.network_required),
            required(review.requires_elevation)
        )),
        Line::from(format!(
            "capabilities: {}",
            if review.capabilities.is_empty() {
                "none".to_string()
            } else {
                review
                    .capabilities
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        )),
        Line::from(format!("rollback: {}", review.rollback)),
    ]
}

fn required(value: bool) -> &'static str {
    if value { "required" } else { "not required" }
}

fn has_review(record: &UiRecord) -> bool {
    record.action_refs.iter().any(|action| {
        matches!(
            action.disposition,
            ActionDisposition::Reviewable | ActionDisposition::Blocked
        )
    }) || record
        .review_boundary
        .as_ref()
        .is_some_and(|boundary| boundary.disposition != ActionDisposition::Unavailable)
}

fn has_action_plan(record: &UiRecord) -> bool {
    record.action_refs.iter().any(|action| {
        matches!(
            action.disposition,
            ActionDisposition::Reviewable | ActionDisposition::Blocked
        )
    })
}

fn view_state_label(state: &ViewState) -> (&'static str, Tone) {
    match state {
        ViewState::Loading { .. } => ("loading", Tone::Accent),
        ViewState::Refreshing { .. } => ("refreshing", Tone::Accent),
        ViewState::Ready { .. } => ("ready", Tone::Safe),
        ViewState::Unavailable { .. } => ("unavailable", Tone::Warn),
        ViewState::Empty { .. } => ("empty", Tone::Info),
        ViewState::Blocked { .. } => ("blocked", Tone::Danger),
        ViewState::Stale { .. } => ("stale", Tone::Warn),
        ViewState::Cancelled { .. } => ("cancelled", Tone::Warn),
        ViewState::Verified { .. } => ("verified", Tone::Safe),
        ViewState::RecoveryRequired { .. } => ("recovery-required", Tone::Warn),
        ViewState::Failed { .. } => ("failed", Tone::Danger),
    }
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
    fn home_shell_is_calm_and_has_no_route_bar_or_numeric_shortcuts() {
        let state = UiState::new(fixture_model());
        let mut terminal = Terminal::new(TestBackend::new(118, 30)).expect("test terminal");
        terminal
            .draw(|frame| draw_shell(frame, &state, false))
            .expect("draw");
        let text = frame_text(terminal.backend().buffer());
        for label in [
            "runtime.zero",
            "HOME",
            "Next safe actions",
            "Selected evidence",
        ] {
            assert!(text.contains(label), "missing {label} in {text}");
        }
        assert!(!text.contains("[1 Overview]"));
        assert!(!text.contains("Modules"));
        assert!(text.contains("Enter inspect"));
    }

    #[test]
    fn semantic_states_render_without_color() {
        let mut state = UiState::new(fixture_model());
        state.apply_event(super::super::messages::UiEvent::JobSucceeded {
            receipt: super::super::model::BoundedId::try_new("receipt/1").expect("id"),
            verification: super::super::model::BoundedId::try_new("verify/1").expect("id"),
        });
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("test terminal");
        terminal
            .draw(|frame| draw_shell(frame, &state, false))
            .expect("draw");
        let text = frame_text(terminal.backend().buffer());
        assert!(text.contains("verified"));
        assert!(text.contains("receipt/1"));
    }
}
