use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchMode {
    TuiDashboard,
    CliDashboardText,
    CliDashboardJson,
    CliSubcommand,
    TuiUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LaunchRoutingReport {
    pub launch_mode: LaunchMode,
    pub interface: InterfaceMode,
    pub stdin_is_tty: bool,
    pub stdout_is_tty: bool,
    pub tui_available: bool,
    pub json_requested: bool,
    pub no_tui_requested: bool,
    pub tui_requested: bool,
    pub automation_detected: bool,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceMode {
    Cli,
    Tui,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaunchEnvironment {
    pub stdin_is_tty: bool,
    pub stdout_is_tty: bool,
    pub tui_available: bool,
    pub automation_detected: bool,
}

impl LaunchEnvironment {
    pub const fn cli_subcommand() -> Self {
        Self {
            stdin_is_tty: false,
            stdout_is_tty: false,
            tui_available: false,
            automation_detected: true,
        }
    }
}

pub fn resolve_launch_mode(args: &[String], env: LaunchEnvironment) -> LaunchRoutingReport {
    let json_requested = json_requested(args);
    let no_tui_requested = args.iter().any(|arg| arg == "--no-tui");
    let tui_requested = args.iter().any(|arg| arg == "--tui");
    let has_subcommand = args.first().is_some_and(|arg| !arg.starts_with('-'));
    let cli_passthrough_flag = args
        .iter()
        .any(|arg| matches!(arg.as_str(), "--help" | "-h" | "--version" | "-V"));

    let (launch_mode, reason) = if has_subcommand || cli_passthrough_flag {
        (
            LaunchMode::CliSubcommand,
            "explicit commands and passthrough flags use scriptable CLI",
        )
    } else if json_requested {
        (
            LaunchMode::CliDashboardJson,
            "JSON output never launches TUI",
        )
    } else if no_tui_requested {
        (LaunchMode::CliDashboardText, "--no-tui bypass requested")
    } else if tui_requested && (!env.stdin_is_tty || !env.stdout_is_tty || env.automation_detected)
    {
        (
            LaunchMode::TuiUnavailable,
            "TUI explicitly requested but terminal is non-interactive or automated",
        )
    } else if !env.stdin_is_tty || !env.stdout_is_tty || env.automation_detected {
        (
            LaunchMode::CliDashboardText,
            "non-interactive or automation context",
        )
    } else if tui_requested && !env.tui_available {
        (
            LaunchMode::TuiUnavailable,
            "TUI explicitly requested but unavailable",
        )
    } else if env.tui_available {
        (
            LaunchMode::TuiDashboard,
            "interactive terminal with TUI available",
        )
    } else {
        (
            LaunchMode::CliDashboardText,
            "TUI not implemented or unavailable",
        )
    };

    LaunchRoutingReport {
        launch_mode,
        interface: interface_for(launch_mode),
        stdin_is_tty: env.stdin_is_tty,
        stdout_is_tty: env.stdout_is_tty,
        tui_available: env.tui_available,
        json_requested,
        no_tui_requested,
        tui_requested,
        automation_detected: env.automation_detected,
        reason,
    }
}

pub fn cli_subcommand_report(command: &'static str) -> LaunchRoutingReport {
    let mut report =
        resolve_launch_mode(&[command.to_string()], LaunchEnvironment::cli_subcommand());
    report.reason = "current command is an explicit CLI subcommand";
    report
}

fn interface_for(launch_mode: LaunchMode) -> InterfaceMode {
    match launch_mode {
        LaunchMode::TuiDashboard => InterfaceMode::Tui,
        LaunchMode::CliDashboardText
        | LaunchMode::CliDashboardJson
        | LaunchMode::CliSubcommand
        | LaunchMode::TuiUnavailable => InterfaceMode::Cli,
    }
}

fn json_requested(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--json")
        || args
            .windows(2)
            .any(|window| window[0] == "--format" && window[1] == "json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interactive(tui_available: bool) -> LaunchEnvironment {
        LaunchEnvironment {
            stdin_is_tty: true,
            stdout_is_tty: true,
            tui_available,
            automation_detected: false,
        }
    }

    #[test]
    fn bare_rz0_uses_tui_when_interactive_and_available() {
        let report = resolve_launch_mode(&[], interactive(true));
        assert_eq!(report.launch_mode, LaunchMode::TuiDashboard);
        assert_eq!(report.interface, InterfaceMode::Tui);
    }

    #[test]
    fn subcommand_uses_scriptable_cli() {
        let report = resolve_launch_mode(&["modules".to_string()], interactive(true));
        assert_eq!(report.launch_mode, LaunchMode::CliSubcommand);
        assert_eq!(report.interface, InterfaceMode::Cli);
    }

    #[test]
    fn help_flag_uses_scriptable_cli() {
        let report = resolve_launch_mode(&["--help".to_string()], interactive(true));
        assert_eq!(report.launch_mode, LaunchMode::CliSubcommand);
    }

    #[test]
    fn json_never_launches_tui() {
        let report = resolve_launch_mode(&["--json".to_string()], interactive(true));
        assert_eq!(report.launch_mode, LaunchMode::CliDashboardJson);
    }

    #[test]
    fn no_tui_bypasses_tui() {
        let report = resolve_launch_mode(&["--no-tui".to_string()], interactive(true));
        assert_eq!(report.launch_mode, LaunchMode::CliDashboardText);
    }

    #[test]
    fn explicit_tui_errors_when_non_interactive() {
        let report = resolve_launch_mode(
            &["--tui".to_string()],
            LaunchEnvironment {
                stdin_is_tty: true,
                stdout_is_tty: false,
                tui_available: true,
                automation_detected: false,
            },
        );
        assert_eq!(report.launch_mode, LaunchMode::TuiUnavailable);
        assert_eq!(report.interface, InterfaceMode::Cli);
    }

    #[test]
    fn non_interactive_stdout_uses_cli() {
        let report = resolve_launch_mode(
            &[],
            LaunchEnvironment {
                stdin_is_tty: true,
                stdout_is_tty: false,
                tui_available: true,
                automation_detected: false,
            },
        );
        assert_eq!(report.launch_mode, LaunchMode::CliDashboardText);
    }
}
