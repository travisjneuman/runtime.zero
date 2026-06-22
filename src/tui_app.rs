use std::io::{self, BufRead, Write};

use crate::launch_routing::LaunchRoutingReport;
use crate::tui_dashboard;
use crate::tui_render::render_dashboard;

pub fn run_interactive_tui(launch_context: &LaunchRoutingReport) -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut input = stdin.lock();
    run_tui_session(&mut input, &mut stdout, launch_context, color_enabled())
}

pub fn run_tui_session<R: BufRead, W: Write>(
    input: &mut R,
    output: &mut W,
    launch_context: &LaunchRoutingReport,
    color: bool,
) -> io::Result<()> {
    let dashboard = tui_dashboard::dashboard();
    write!(output, "\x1b[2J\x1b[H")?;
    write!(output, "{}", render_dashboard(&dashboard, color))?;
    writeln!(output, "press q then Enter to quit")?;
    writeln!(
        output,
        "launch_mode: {:?} | reason: {}",
        launch_context.launch_mode, launch_context.reason
    )?;
    output.flush()?;
    wait_for_exit(input)
}

fn wait_for_exit<R: BufRead>(input: &mut R) -> io::Result<()> {
    let mut buffer = String::new();
    loop {
        buffer.clear();
        let bytes = input.read_line(&mut buffer)?;
        if bytes == 0 || matches!(buffer.trim(), "" | "q" | "Q") {
            break;
        }
    }
    Ok(())
}

fn color_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launch_routing::{LaunchEnvironment, resolve_launch_mode};

    #[test]
    fn tui_session_renders_and_exits_on_q() {
        let args: Vec<String> = Vec::new();
        let launch = resolve_launch_mode(
            &args,
            LaunchEnvironment {
                stdin_is_tty: true,
                stdout_is_tty: true,
                tui_available: true,
                automation_detected: false,
            },
        );
        let mut input = io::Cursor::new("q\n");
        let mut output = Vec::new();
        run_tui_session(&mut input, &mut output, &launch, false).expect("TUI should render");
        let rendered = String::from_utf8(output).expect("output should be valid UTF-8");
        assert!(rendered.contains("runtime.zero"));
        assert!(rendered.contains("read-only:"));
        assert!(rendered.contains("press q then Enter to quit"));
    }
}
