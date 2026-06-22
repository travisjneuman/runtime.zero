use crate::tui_theme;

pub(crate) fn line(
    content: &str,
    width: usize,
    color: bool,
    tone: Option<tui_theme::TuiTone>,
) -> String {
    let content = truncate(content, width.saturating_sub(4));
    let styled = tone
        .map(|tone| colorize(&content, tone, color))
        .unwrap_or_else(|| content.clone());
    let padding = width.saturating_sub(content.chars().count() + 4);
    format!("│ {}{} │", styled, " ".repeat(padding))
}

pub(crate) fn line_plain(content: &str, width: usize) -> String {
    line(content, width, false, None)
}

pub(crate) fn split_line_toned(
    left: &str,
    right: &str,
    left_width: usize,
    right_width: usize,
    left_tone: Option<tui_theme::TuiTone>,
    right_tone: Option<tui_theme::TuiTone>,
    color: bool,
) -> String {
    let left = pad(&truncate(left, left_width), left_width);
    let right = pad(&truncate(right, right_width), right_width);
    format!(
        "│ {} │ {} │",
        colorize_optional(&left, left_tone, color),
        colorize_optional(&right, right_tone, color)
    )
}

pub(crate) fn border_top(width: usize) -> String {
    format!("╭{}╮", "─".repeat(width - 2))
}

pub(crate) fn border_bottom(width: usize) -> String {
    format!("╰{}╯", "─".repeat(width - 2))
}

pub(crate) fn separator(width: usize) -> String {
    format!("├{}┤", "─".repeat(width - 2))
}

pub(crate) fn pad(value: &str, width: usize) -> String {
    let len = value.chars().count();
    if len >= width {
        value.to_string()
    } else {
        format!("{}{}", value, " ".repeat(width - len))
    }
}

pub(crate) fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let keep = max_chars.saturating_sub(1);
    let mut output: String = value.chars().take(keep).collect();
    output.push('…');
    output
}

fn colorize_optional(content: &str, tone: Option<tui_theme::TuiTone>, color: bool) -> String {
    tone.map(|tone| colorize(content, tone, color))
        .unwrap_or_else(|| content.to_string())
}

pub(crate) fn colorize(content: &str, tone: tui_theme::TuiTone, color: bool) -> String {
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
