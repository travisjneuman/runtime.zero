use crate::{ExitCode, tui_dashboard, tui_render};

pub fn dashboard_text() -> (ExitCode, String, String) {
    (
        ExitCode::Ok,
        tui_render::render_dashboard(&tui_dashboard::dashboard(), false),
        String::new(),
    )
}

pub fn dashboard_json() -> (ExitCode, String, String) {
    match serde_json::to_string_pretty(&tui_dashboard::dashboard()) {
        Ok(json) => (ExitCode::Ok, format!("{json}\n"), String::new()),
        Err(err) => (ExitCode::Usage, String::new(), err.to_string()),
    }
}
