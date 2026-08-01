use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::tui_command_rail::{COMMANDS, selected_command};
use crate::tui_ratatui_components::details_line;
use crate::tui_ratatui_support::{block, command_line, label_line};
use crate::tui_state::{TuiFocusRegion, TuiState};
use crate::tui_theme;

pub(crate) fn render_command_rail(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &TuiState,
    color: bool,
) {
    let panel = block("COMMANDS", "accent", color);
    let inner = panel.inner(area);
    frame.render_widget(panel, area);

    let selected = state.selected_command.min(COMMANDS.len().saturating_sub(1));
    let show_details = state.preview_open && state.focus_region == TuiFocusRegion::CommandRail;
    let detail_height = if show_details { 2 } else { 1 }.min(inner.height);
    let detail_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: detail_height,
    };
    let detail_lines = if show_details {
        let command = selected_command(selected);
        vec![
            details_line(color),
            Line::raw(format!("{} · {}", command.preview, command.command)),
        ]
    } else {
        vec![label_line(
            tui_theme::LABEL_INFO,
            "available command",
            "info",
            color,
        )]
    };
    frame.render_widget(Paragraph::new(detail_lines), detail_area);

    let list_area = Rect {
        x: inner.x,
        y: inner.y.saturating_add(detail_height),
        width: inner.width,
        height: inner.height.saturating_sub(detail_height),
    };
    let rows = COMMANDS
        .iter()
        .enumerate()
        .map(|(index, command)| {
            command_line(
                *command,
                state.focus_region == TuiFocusRegion::CommandRail && index == selected,
                color,
            )
        })
        .collect::<Vec<_>>();
    let list_height = usize::from(list_area.height);
    let max_scroll = rows.len().saturating_sub(list_height);
    let scroll = selected
        .saturating_sub(list_height.saturating_sub(1))
        .min(max_scroll);
    frame.render_widget(
        Paragraph::new(rows).scroll((u16::try_from(scroll).unwrap_or(u16::MAX), 0)),
        list_area,
    );
}
