use crate::tui_dashboard::{TuiDashboard, TuiRow};
use crate::tui_state::TuiState;
use crate::tui_theme;

const WIDTH: usize = 74;
const MIN_WIDTH: usize = 48;

pub fn render_dashboard(dashboard: &TuiDashboard, color: bool) -> String {
    render_dashboard_frame(dashboard, color, WIDTH as u16, 40, &TuiState::new(0), false)
}

pub fn render_dashboard_with_state(
    dashboard: &TuiDashboard,
    color: bool,
    width: u16,
    height: u16,
    state: &TuiState,
) -> String {
    render_dashboard_frame(dashboard, color, width, height, state, true)
}

fn render_dashboard_frame(
    dashboard: &TuiDashboard,
    color: bool,
    width: u16,
    height: u16,
    state: &TuiState,
    interactive: bool,
) -> String {
    let width = usize::from(width).clamp(MIN_WIDTH, 120);
    let height = usize::from(height).max(12);
    let mut out = String::new();
    out.push_str(&border_top(width));
    out.push('\n');
    out.push_str(&line(
        &format!(
            "{} {}  v{}",
            dashboard.title, dashboard.command, dashboard.version
        ),
        color,
        Some(tui_theme::TuiTone::Accent),
        width,
    ));
    out.push('\n');
    out.push_str(&line(
        dashboard.mode,
        color,
        Some(tui_theme::TuiTone::Info),
        width,
    ));
    out.push('\n');
    out.push_str(&separator(width));
    out.push('\n');

    let help_lines = if interactive && state.show_help {
        5
    } else if interactive {
        2
    } else {
        2
    };
    let mut remaining = height.saturating_sub(help_lines + 6);
    for (index, section) in dashboard.sections.iter().enumerate() {
        if remaining == 0 {
            break;
        }
        let marker = if interactive && index == state.selected_section {
            ">"
        } else {
            " "
        };
        out.push_str(&line(
            &format!("{marker} {} /", section.title),
            color,
            Some(tui_theme::TuiTone::Muted),
            width,
        ));
        out.push('\n');
        remaining = remaining.saturating_sub(1);
        for row in &section.rows {
            if remaining == 0 {
                break;
            }
            out.push_str(&line(&format_row(row, false, width), false, None, width));
            out.push('\n');
            remaining = remaining.saturating_sub(1);
        }
    }

    out.push_str(&separator(width));
    out.push('\n');
    out.push_str(&line(
        "read-only: no installs, cleanup, module execution, or store writes",
        color,
        Some(tui_theme::TuiTone::DryRun),
        width,
    ));
    out.push('\n');
    write_help(&mut out, interactive, state.show_help, width);
    out.push('\n');
    out.push_str(&border_bottom(width));
    out.push('\n');
    out
}

fn format_row(row: &TuiRow, color: bool, width: usize) -> String {
    let label = format!("{:<19}", row.label);
    let label = colorize(&label, tone_for(row.tone), color);
    let value_width = width.saturating_sub(27);
    let value = truncate(&row.value, value_width);
    format!("  {label} {value}")
}

fn line(content: &str, color: bool, tone: Option<tui_theme::TuiTone>, width: usize) -> String {
    let content = truncate(content, width.saturating_sub(4));
    let styled = tone
        .map(|tone| colorize(&content, tone, color))
        .unwrap_or_else(|| content.clone());
    let plain_len = content.chars().count();
    let padding = width.saturating_sub(plain_len + 4);
    format!(
        "{} {}{} {}",
        tui_theme::BORDER_VERTICAL,
        styled,
        " ".repeat(padding),
        tui_theme::BORDER_VERTICAL
    )
}

fn write_help(out: &mut String, interactive: bool, expanded: bool, width: usize) {
    if !interactive {
        out.push_str(&line(
            "commands: rz0 doctor | rz0 store status | rz0 store init --dry-run",
            false,
            None,
            width,
        ));
    } else if expanded {
        out.push_str(&line(
            "keys: q/Esc quit | h/? help | Tab/↓ next | ↑ previous",
            false,
            None,
            width,
        ));
        out.push('\n');
        out.push_str(&line(
            "safe shell: review-only dashboard; all mutations stay behind explicit CLI",
            false,
            None,
            width,
        ));
    } else {
        out.push_str(&line(
            "keys: q quit | h help | Tab/↓ next section",
            false,
            None,
            width,
        ));
    }
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

fn border_top(width: usize) -> String {
    format!(
        "{}{}{}",
        tui_theme::BORDER_TOP_LEFT,
        tui_theme::BORDER_HORIZONTAL.repeat(width - 2),
        tui_theme::BORDER_TOP_RIGHT
    )
}

fn border_bottom(width: usize) -> String {
    format!(
        "{}{}{}",
        tui_theme::BORDER_BOTTOM_LEFT,
        tui_theme::BORDER_HORIZONTAL.repeat(width - 2),
        tui_theme::BORDER_BOTTOM_RIGHT
    )
}

fn separator(width: usize) -> String {
    format!(
        "{}{}{}",
        tui_theme::BORDER_VERTICAL,
        tui_theme::BORDER_HORIZONTAL.repeat(width - 2),
        tui_theme::BORDER_VERTICAL
    )
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let keep = max_chars.saturating_sub(1);
    let mut output: String = value.chars().take(keep).collect();
    output.push('…');
    output
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
        assert!(rendered.contains("rz0 store status"));
        assert!(!rendered.contains("\x1b["));
    }

    #[test]
    fn render_handles_narrow_terminal_and_help() {
        let mut state = TuiState::new(3);
        state.show_help = true;
        let rendered =
            render_dashboard_with_state(&tui_dashboard::dashboard(), false, 32, 12, &state);
        assert!(rendered.contains("q/Esc quit"));
        assert!(!rendered.contains("\x1b["));
    }
}
