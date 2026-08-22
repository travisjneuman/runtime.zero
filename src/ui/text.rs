//! Deterministic scriptable text projection of the typed UI model.
//!
//! This is the `--no-tui` projection, not an interactive renderer. It shares
//! the same foundation evidence and action references as the terminal flow.

use std::collections::BTreeSet;

use super::model::{RecordStatus, Route, UiModel};
use crate::tui_theme;

const WIDTH: usize = 86;

pub fn render_dashboard(model: &UiModel, color: bool) -> String {
    let overview = model.route(Route::Overview);
    let review = model.route(Route::Review);
    let mut attention_keys = BTreeSet::new();
    let attention = overview
        .records
        .iter()
        .chain(review.records.iter())
        .filter(|record| {
            let attention = matches!(
                record.status,
                RecordStatus::Plan
                    | RecordStatus::Warn
                    | RecordStatus::Blocked
                    | RecordStatus::Error
            ) || record.action_refs.iter().any(|action| {
                matches!(
                    action.disposition,
                    super::model::ActionDisposition::Reviewable
                        | super::model::ActionDisposition::Blocked
                )
            });
            attention && attention_keys.insert(format!("{}\n{}", record.title, record.summary))
        })
        .collect::<Vec<_>>();
    let next = attention
        .first()
        .copied()
        .or_else(|| overview.records.first());
    let mut lines = vec![
        border_top(WIDTH),
        line(
            "runtime.zero · Home · local snapshot",
            WIDTH,
            color,
            Tone::Accent,
        ),
        line(
            &format!(
                "state: {} · local foundation evidence · read-only",
                model.state.label()
            ),
            WIDTH,
            color,
            Tone::Info,
        ),
        separator(WIDTH),
        line("Home / next safe action", WIDTH, color, Tone::Accent),
        line(
            &format!(
                "{} · {} attention item{}",
                if attention.is_empty() {
                    "nothing needs attention"
                } else {
                    "review required"
                },
                attention.len(),
                if attention.len() == 1 { "" } else { "s" }
            ),
            WIDTH,
            color,
            if attention.is_empty() {
                Tone::Safe
            } else {
                Tone::Warn
            },
        ),
    ];
    if let Some(record) = next {
        lines.push(line(
            &format!("next: {} {}", record.status.label(), record.title),
            WIDTH,
            color,
            tone_for_status(record.status),
        ));
        lines.push(line_plain(record.summary.as_str(), WIDTH));
    } else {
        lines.push(line_plain(
            "No evidence is available in this workspace.",
            WIDTH,
        ));
    }
    lines.extend([
        separator(WIDTH),
        line("Attention", WIDTH, color, Tone::Accent),
    ]);
    if attention.is_empty() {
        lines.push(line_plain(
            "No blocked or reviewable item was reported.",
            WIDTH,
        ));
    } else {
        for (index, record) in attention.iter().take(8).enumerate() {
            lines.push(line(
                &format!(
                    "{} {} {}",
                    if index == 0 { ">" } else { " " },
                    record.status.label(),
                    record.title
                ),
                WIDTH,
                color,
                tone_for_status(record.status),
            ));
        }
    }
    lines.extend([
        separator(WIDTH),
        line("CLI escape hatch", WIDTH, color, Tone::Accent),
        line_plain(
            "rz0 --no-tui · rz0 doctor · rz0 scan --dry-run · rz0 --json",
            WIDTH,
        ),
        line(
            &format!("status  {}", model.status),
            WIDTH,
            color,
            Tone::Info,
        ),
        border_bottom(WIDTH),
    ]);
    lines.join("\n") + "\n"
}

#[derive(Clone, Copy)]
enum Tone {
    Accent,
    Info,
    Safe,
    DryRun,
    Warn,
    Muted,
}

fn tone_for_status(status: RecordStatus) -> Tone {
    match status {
        RecordStatus::Ok => Tone::Safe,
        RecordStatus::Info | RecordStatus::Observed => Tone::Info,
        RecordStatus::Plan => Tone::Accent,
        RecordStatus::DryRun => Tone::DryRun,
        RecordStatus::Warn | RecordStatus::Blocked | RecordStatus::Error => Tone::Warn,
        RecordStatus::Muted => Tone::Muted,
    }
}

fn line(content: &str, width: usize, color: bool, tone: Tone) -> String {
    let content = truncate(content, width.saturating_sub(4));
    let rendered = if color {
        format!("{}{}{}", ansi(tone), content, tui_theme::ANSI_RESET)
    } else {
        content.clone()
    };
    format!(
        "│ {}{} │",
        rendered,
        " ".repeat(width.saturating_sub(content.chars().count() + 4))
    )
}

fn line_plain(content: &str, width: usize) -> String {
    line(content, width, false, Tone::Muted)
}

fn border_top(width: usize) -> String {
    format!("╭{}╮", "─".repeat(width.saturating_sub(2)))
}

fn border_bottom(width: usize) -> String {
    format!("╰{}╯", "─".repeat(width.saturating_sub(2)))
}

fn separator(width: usize) -> String {
    format!("├{}┤", "─".repeat(width.saturating_sub(2)))
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut output: String = value.chars().take(max_chars.saturating_sub(1)).collect();
    output.push('…');
    output
}

fn ansi(tone: Tone) -> &'static str {
    match tone {
        Tone::Accent => tui_theme::ansi(tui_theme::TuiTone::Accent),
        Tone::Info => tui_theme::ansi(tui_theme::TuiTone::Info),
        Tone::Safe => tui_theme::ansi(tui_theme::TuiTone::Safe),
        Tone::DryRun => tui_theme::ansi(tui_theme::TuiTone::DryRun),
        Tone::Warn => tui_theme::ansi(tui_theme::TuiTone::Warn),
        Tone::Muted => tui_theme::ansi(tui_theme::TuiTone::Muted),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::testkit::fixture_model;

    #[test]
    fn text_projection_is_deterministic_and_color_toggle_is_explicit() {
        let model = fixture_model();
        let first = render_dashboard(&model, false);
        assert_eq!(first, render_dashboard(&model, false));
        assert!(first.contains("Home / next safe action"));
        assert!(first.contains("CLI escape hatch"));
        assert!(!first.contains("\u{1b}["));
        assert!(render_dashboard(&model, true).contains("\u{1b}["));
    }
}
