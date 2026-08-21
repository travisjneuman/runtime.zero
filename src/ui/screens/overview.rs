use ratatui::Frame;

use super::super::state::UiState;
use super::super::widgets;

pub(crate) fn draw(frame: &mut Frame<'_>, state: &UiState, color: bool) {
    widgets::draw_shell(frame, state, color);
}
