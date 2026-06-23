use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::tui_command_rail::{COMMANDS, selected_command};
use crate::tui_ratatui_support::{block, command_line, focused_title, tone_style};
use crate::tui_state::{TuiFocusRegion, TuiState};

pub(crate) fn render_command_rail(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &TuiState,
    color: bool,
) {
    let mut lines = Vec::new();
    if state.preview_open && state.focus_region == TuiFocusRegion::CommandRail {
        let command = selected_command(state.selected_command);
        lines.push(Line::styled(
            format!("PREVIEW · {}", command.preview),
            tone_style("accent", color),
        ));
        lines.push(Line::raw(
            "not executed from TUI; copy/run manually if desired",
        ));
        lines.push(Line::raw(""));
    }
    for (index, command) in COMMANDS.iter().enumerate() {
        let focused = state.focus_region == TuiFocusRegion::CommandRail
            && index == state.selected_command.min(COMMANDS.len().saturating_sub(1));
        lines.push(command_line(*command, focused, color));
    }
    frame.render_widget(
        Paragraph::new(lines).block(block(
            focused_title(
                "SCRIPTABLE CLI RAIL",
                state.focus_region == TuiFocusRegion::CommandRail,
            ),
            "accent",
            color,
        )),
        area,
    );
}
