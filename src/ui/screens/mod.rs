use ratatui::Frame;

use super::state::UiState;
use crate::ui::model::Route;

mod activity;
mod explore;
mod modules;
mod overview;
mod review;

pub(crate) fn draw(frame: &mut Frame<'_>, state: &UiState, color: bool) {
    match state.route {
        Route::Overview => overview::draw(frame, state, color),
        Route::Explore => explore::draw(frame, state, color),
        Route::Review => review::draw(frame, state, color),
        Route::Activity => activity::draw(frame, state, color),
        Route::Modules => modules::draw(frame, state, color),
    }
}
