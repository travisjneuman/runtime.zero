use std::env;
use std::io::{self, IsTerminal};
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let launch_context =
        runtime_zero::launch_routing::resolve_launch_mode(&args, launch_environment());

    if launch_context.launch_mode == runtime_zero::launch_routing::LaunchMode::TuiDashboard {
        if let Err(err) = runtime_zero::tui_app::run_interactive_tui(&launch_context) {
            eprintln!("failed to render TUI: {err}");
            process::exit(runtime_zero::ExitCode::Usage.as_i32());
        }
        process::exit(runtime_zero::ExitCode::Ok.as_i32());
    }

    let (code, stdout, stderr) = if dashboard_json_requested(&launch_context) {
        runtime_zero::dashboard_cli::dashboard_json()
    } else if dashboard_text_requested(&args, &launch_context) {
        runtime_zero::dashboard_cli::dashboard_text()
    } else {
        runtime_zero::run(args)
    };

    if !stdout.is_empty() {
        print!("{stdout}");
    }

    if !stderr.is_empty() {
        eprint!("{stderr}");
    }

    process::exit(code.as_i32());
}

fn launch_environment() -> runtime_zero::launch_routing::LaunchEnvironment {
    runtime_zero::launch_routing::LaunchEnvironment {
        stdin_is_tty: io::stdin().is_terminal(),
        stdout_is_tty: io::stdout().is_terminal(),
        tui_available: true,
        automation_detected: automation_detected(),
    }
}

fn automation_detected() -> bool {
    ["CI", "GITHUB_ACTIONS", "TF_BUILD", "TEAMCITY_VERSION"]
        .iter()
        .any(|name| env::var_os(name).is_some())
}

fn dashboard_json_requested(
    launch_context: &runtime_zero::launch_routing::LaunchRoutingReport,
) -> bool {
    launch_context.launch_mode == runtime_zero::launch_routing::LaunchMode::CliDashboardJson
}

fn dashboard_text_requested(
    args: &[String],
    launch_context: &runtime_zero::launch_routing::LaunchRoutingReport,
) -> bool {
    args.is_empty()
        || matches!(
            launch_context.launch_mode,
            runtime_zero::launch_routing::LaunchMode::CliDashboardText
                | runtime_zero::launch_routing::LaunchMode::TuiUnavailable
        )
}
