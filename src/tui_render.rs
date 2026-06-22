use crate::tui_dashboard::{TuiDashboard, TuiRow};
use crate::tui_theme;

const WIDTH: usize = 74;

pub fn render_dashboard(dashboard: &TuiDashboard, color: bool) -> String {
    let mut out = String::new();
    out.push_str(&border_top());
    out.push('\n');
    out.push_str(&line(
        &format!(
            "{} {}  v{}",
            dashboard.title, dashboard.command, dashboard.version
        ),
        color,
        Some(tui_theme::TuiTone::Accent),
    ));
    out.push('\n');
    out.push_str(&line(dashboard.mode, color, Some(tui_theme::TuiTone::Info)));
    out.push('\n');
    out.push_str(&separator());
    out.push('\n');

    for section in &dashboard.sections {
        out.push_str(&line(
            &format!("{} /", section.title),
            color,
            Some(tui_theme::TuiTone::Muted),
        ));
        out.push('\n');
        for row in &section.rows {
            out.push_str(&line(&format_row(row, color), false, None));
            out.push('\n');
        }
    }

    out.push_str(&separator());
    out.push('\n');
    out.push_str(&line(
        "read-only: no installs, cleanup, module execution, or store writes",
        color,
        Some(tui_theme::TuiTone::DryRun),
    ));
    out.push('\n');
    out.push_str(&line(
        "commands: rz0 doctor | rz0 store status | rz0 store init --dry-run",
        false,
        None,
    ));
    out.push('\n');
    out.push_str(&border_bottom());
    out.push('\n');
    out
}

fn format_row(row: &TuiRow, color: bool) -> String {
    let label = colorize(row.label, tone_for(row.tone), color);
    format!("  {label:<19} {}", row.value)
}

fn line(content: &str, color: bool, tone: Option<tui_theme::TuiTone>) -> String {
    let styled = tone
        .map(|tone| colorize(content, tone, color))
        .unwrap_or_else(|| content.to_string());
    let plain_len = content.chars().count();
    let padding = WIDTH.saturating_sub(plain_len + 4);
    format!(
        "{} {}{} {}",
        tui_theme::BORDER_VERTICAL,
        styled,
        " ".repeat(padding),
        tui_theme::BORDER_VERTICAL
    )
}

fn colorize(content: &str, tone: tui_theme::TuiTone, color: bool) -> String {
    if color {
        format!(
            "{}{}{}",
            tui_theme::ansi(tone),
            content,
            tui_theme::ANSI_RESET
        )
    } else {
        content.to_string()
    }
}

fn tone_for(tone: &str) -> tui_theme::TuiTone {
    match tone {
        "accent" => tui_theme::TuiTone::Accent,
        "safe" => tui_theme::TuiTone::Safe,
        "dry_run" => tui_theme::TuiTone::DryRun,
        "warn" => tui_theme::TuiTone::Warn,
        "muted" => tui_theme::TuiTone::Muted,
        _ => tui_theme::TuiTone::Info,
    }
}

fn border_top() -> String {
    format!(
        "{}{}{}",
        tui_theme::BORDER_TOP_LEFT,
        tui_theme::BORDER_HORIZONTAL.repeat(WIDTH - 2),
        tui_theme::BORDER_TOP_RIGHT
    )
}

fn border_bottom() -> String {
    format!(
        "{}{}{}",
        tui_theme::BORDER_BOTTOM_LEFT,
        tui_theme::BORDER_HORIZONTAL.repeat(WIDTH - 2),
        tui_theme::BORDER_BOTTOM_RIGHT
    )
}

fn separator() -> String {
    format!(
        "{}{}{}",
        tui_theme::BORDER_VERTICAL,
        tui_theme::BORDER_HORIZONTAL.repeat(WIDTH - 2),
        tui_theme::BORDER_VERTICAL
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui_dashboard;

    #[test]
    fn render_plain_dashboard_without_ansi() {
        let rendered = render_dashboard(&tui_dashboard::dashboard(), false);
        assert!(rendered.contains("runtime.zero rz0"));
        assert!(rendered.contains("read-only:"));
        assert!(!rendered.contains("\x1b["));
    }
}
