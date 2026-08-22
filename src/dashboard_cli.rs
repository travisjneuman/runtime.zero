use crate::{ExitCode, tui_dashboard};

pub fn dashboard_text() -> (ExitCode, String, String) {
    dashboard_text_with_color(false)
}

pub fn dashboard_text_with_color(color: bool) -> (ExitCode, String, String) {
    let dashboard = tui_dashboard::dashboard();
    let model = crate::ui::foundation_adapter::model_from_dashboard(&dashboard, 0);
    (
        ExitCode::Ok,
        crate::ui::text::render_dashboard(&model, color),
        String::new(),
    )
}

pub fn dashboard_json() -> (ExitCode, String, String) {
    match serde_json::to_string_pretty(&tui_dashboard::private_dashboard()) {
        Ok(json) => (ExitCode::Ok, format!("{json}\n"), String::new()),
        Err(err) => (ExitCode::Usage, String::new(), err.to_string()),
    }
}
