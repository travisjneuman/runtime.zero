use std::env;
use std::io::{self, ErrorKind, IsTerminal, Write};
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let parsed = match runtime_zero::color_mode::parse_global_args(&args) {
        Ok(parsed) => parsed,
        Err((code, message)) => {
            write_output_or_exit(OutputStream::Stderr, &message);
            process::exit(code.as_i32());
        }
    };
    let args = parsed.args;
    let launch_context =
        runtime_zero::launch_routing::resolve_launch_mode(&args, launch_environment());

    if launch_context.launch_mode == runtime_zero::launch_routing::LaunchMode::TuiDashboard {
        let color = parsed
            .color_mode
            .enabled_for_tui(launch_context.stdout_is_tty);
        if let Err(err) = runtime_zero::tui_app::run_interactive_tui(&launch_context, color) {
            write_output_or_exit(
                OutputStream::Stderr,
                &format!("failed to render TUI: {err}\n"),
            );
            process::exit(runtime_zero::ExitCode::Usage.as_i32());
        }
        process::exit(runtime_zero::ExitCode::Ok.as_i32());
    }

    if launch_context.launch_mode == runtime_zero::launch_routing::LaunchMode::TuiUnavailable {
        write_output_or_exit(
            OutputStream::Stderr,
            &format!("TUI requested but unavailable: {}\n", launch_context.reason),
        );
        process::exit(runtime_zero::ExitCode::Usage.as_i32());
    }

    let (code, stdout, stderr) = if dashboard_json_requested(&launch_context) {
        runtime_zero::dashboard_cli::dashboard_json()
    } else if dashboard_text_requested(&args, &launch_context) {
        runtime_zero::dashboard_cli::dashboard_text_with_color(
            parsed.color_mode.enabled_for_scriptable_text(),
        )
    } else {
        runtime_zero::run(args)
    };

    if !stdout.is_empty() {
        write_output_or_exit(OutputStream::Stdout, &stdout);
    }

    if !stderr.is_empty() {
        write_output_or_exit(OutputStream::Stderr, &stderr);
    }

    process::exit(code.as_i32());
}

enum OutputStream {
    Stdout,
    Stderr,
}

fn write_output_or_exit(stream: OutputStream, content: &str) {
    let result = match stream {
        OutputStream::Stdout => io::stdout().lock().write_all(content.as_bytes()),
        OutputStream::Stderr => io::stderr().lock().write_all(content.as_bytes()),
    };
    if let Err(err) = result {
        if err.kind() == ErrorKind::BrokenPipe {
            process::exit(runtime_zero::ExitCode::Ok.as_i32());
        }
        let _ = writeln!(io::stderr().lock(), "failed to write output: {err}");
        process::exit(runtime_zero::ExitCode::Usage.as_i32());
    }
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
        )
}
