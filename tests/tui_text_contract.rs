use runtime_zero::dashboard_cli;
use runtime_zero::tui_dashboard;
use runtime_zero::tui_render::render_dashboard;

#[test]
fn scriptable_dashboard_text_keeps_the_public_safety_copy() {
    let rendered = render_dashboard(&tui_dashboard::dashboard(), false);
    assert!(rendered.contains("runtime.zero"));
    assert!(rendered.contains("local snapshot"));
    assert!(rendered.contains("Home / next step"));
    assert!(rendered.contains("status"));
    assert!(!rendered.contains("\x1b["));
}

#[test]
fn scriptable_dashboard_text_is_still_the_no_tui_contract() {
    let (code, stdout, stderr) = dashboard_cli::dashboard_text_with_color(false);
    assert_eq!(code, runtime_zero::ExitCode::Ok);
    assert!(stderr.is_empty());
    assert!(stdout.contains("runtime.zero"));
    assert!(stdout.ends_with('\n'));
    assert!(!stdout.contains("\x1b["));
}

#[test]
fn dashboard_json_remains_machine_readable_and_separate_from_tui() {
    let (code, stdout, stderr) = dashboard_cli::dashboard_json();
    assert_eq!(code, runtime_zero::ExitCode::Ok);
    assert!(stderr.is_empty());
    assert!(stdout.contains("\"contract\": \"foundation_dashboard\""));
    assert!(!stdout.contains("\x1b["));
}
