//! Deterministic scriptable text projection of the typed UI model.
//!
//! This is the `--no-tui`/non-interactive projection, not an interactive
//! renderer. It consumes the same bounded model as Ratatui so the CLI and TUI
//! cannot grow separate evidence or action authorities.

use super::model::{RecordStatus, Route, UiModel};
use crate::tui_theme;

const WIDTH: usize = 86;

pub fn render_dashboard(model: &UiModel, color: bool) -> String {
    let overview = model.route(Route::Overview);
    let mut lines = vec![
        border_top(WIDTH),
        line("runtime.zero · local snapshot", WIDTH, color, Tone::Accent),
        line(
            &format!(
                "{} · {} records · review first",
                model.state.label(),
                overview.records.len()
            ),
            WIDTH,
            color,
            Tone::Info,
        ),
        separator(WIDTH),
        line(
            "• Home / next step  ·  Explore  ·  Review  ·  Activity  ·  Modules",
            WIDTH,
            color,
            Tone::Accent,
        ),
        separator(WIDTH),
        line("Home / next step", WIDTH, color, Tone::Accent),
        line(
            &format!("{} · {}", overview.summary, overview.records.len()),
            WIDTH,
            color,
            Tone::Muted,
        ),
    ];

    if overview.records.is_empty() {
        lines.push(line_plain(
            "No records are available in this workspace.",
            WIDTH,
        ));
    } else {
        for (index, record) in overview.records.iter().enumerate() {
            lines.push(line(
                &format!(
                    "{} {:<9} {}",
                    if index == 0 { "·" } else { " " },
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
        line("Selected", WIDTH, color, Tone::Accent),
    ]);
    if let Some(record) = overview.records.first() {
        lines.push(line(
            &format!("{}  {}", record.status.label(), record.title),
            WIDTH,
            color,
            tone_for_status(record.status),
        ));
        lines.push(line_plain(record.summary.as_str(), WIDTH));
    } else {
        lines.push(line_plain("No evidence is selected.", WIDTH));
    }
    lines.extend([
        separator(WIDTH),
        line(
            &format!("status  {} · read-only typed evidence", model.status),
            WIDTH,
            color,
            Tone::Info,
        ),
        line_plain(
            "commands: rz0 doctor · rz0 apps · rz0 store status · rz0 --json",
            WIDTH,
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
        RecordStatus::Warn => Tone::Warn,
        RecordStatus::Blocked | RecordStatus::Error => Tone::Warn,
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
        assert!(first.contains("Home / next step"));
        assert!(first.contains("status"));
        assert!(!first.contains("\u{1b}["));
        assert!(render_dashboard(&model, true).contains("\u{1b}["));
    }
}
