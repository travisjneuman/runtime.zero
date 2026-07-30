pub mod apps;
pub mod brand;
pub mod color_mode;
pub mod dashboard_cli;
pub mod install_receipt;
mod install_receipt_schema;
pub mod installed_registry;
mod installed_registry_path;
pub mod inventory;
pub mod launch_routing;
pub mod module_cli;
pub mod module_install_plan;
pub mod module_manifest;
pub mod module_registry;
pub mod module_store;
pub mod module_validation;
pub mod package_integrity;
mod package_integrity_io;
pub mod store_cli;
pub mod store_init;
mod store_init_model;
pub mod store_init_text;
pub mod store_plan;
pub mod store_status;
pub mod store_status_text;
pub mod tui_app;
pub mod tui_canvas;
mod tui_command_rail;
pub mod tui_dashboard;
mod tui_dashboard_labels;
pub mod tui_layout;
pub mod tui_ratatui;
mod tui_ratatui_components;
mod tui_ratatui_rail;
mod tui_ratatui_support;
pub mod tui_render;
mod tui_render_support;
pub mod tui_state;
pub mod tui_theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Ok = 0,
    Usage = 2,
}

impl ExitCode {
    pub const fn as_i32(self) -> i32 {
        self as i32
    }
}

pub fn run<I, S>(args: I) -> (ExitCode, String, String)
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args: Vec<String> = args.into_iter().map(Into::into).collect();
    let args = match color_mode::parse_global_args(&args) {
        Ok(parsed) => parsed.args,
        Err((code, message)) => return (code, String::new(), message),
    };

    match args.first().map(String::as_str) {
        None => (ExitCode::Ok, help_text(), String::new()),
        Some("--help" | "-h" | "help") => (ExitCode::Ok, help_text(), String::new()),
        Some("--version" | "-V" | "version") => (ExitCode::Ok, version_text(), String::new()),
        Some("doctor") => doctor_command(&args[1..]),
        Some("apps") => apps::apps_command(&args[1..]),
        Some("uninstall") => apps::uninstall_command(&args[1..]),
        Some("modules") => module_cli::modules_command(&args[1..]),
        Some("store") => store_cli::store_command(&args[1..]),
        Some("scan") => scan_command(&args[1..]),
        Some(command) => unknown_command(command),
    }
}

pub fn version_text() -> String {
    format!(
        "{} {} {}\n{}\n",
        brand::TITLE,
        brand::COMMAND,
        env!("CARGO_PKG_VERSION"),
        brand::SUBTITLE
    )
}

pub fn help_text() -> String {
    format!(
        "{title} — {subtitle}\n\nUsage:\n  {cmd}\n  {cmd} --tui\n  {cmd} --no-tui\n  {cmd} --json\n  {cmd} --color auto|always|never\n  {cmd} --version\n  {cmd} doctor [--format json]\n  {cmd} apps [--format json]\n  {cmd} uninstall plan <installed-software-id> [--format json]\n  {cmd} modules [--format json]\n  {cmd} modules --from <dir> [--format json]\n  {cmd} modules validate <manifest.json> [--format json]\n  {cmd} modules install --dry-run <package-dir-or-manifest> [--format json]\n  {cmd} store plan [--format json]\n  {cmd} store status [--store-root <path>] [--format json]\n  {cmd} store init --dry-run [--format json]\n  {cmd} store init --yes [--format json]\n  {cmd} scan --dry-run [--include-raw-paths] [--format json]\n\nFoundation safety posture:\n  {safety}\n\nThe core includes bounded read-only local inventory, validates local manifests, and lists installed modules. Uninstall remains review-only until an exact quarantine/manager transaction is confirmed and authorized.\n",
        title = brand::TITLE,
        subtitle = brand::SUBTITLE,
        cmd = brand::COMMAND,
        safety = brand::SAFETY_POSTURE
    )
}

pub fn doctor_text() -> String {
    rz0_diagnostics_contract::diagnostic_text(&doctor_report())
}

pub fn doctor_report() -> rz0_diagnostics_contract::DiagnosticReport {
    rz0_diagnostics_contract::foundation_diagnostics(
        brand::TITLE,
        brand::COMMAND,
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
    )
}

fn doctor_command(args: &[String]) -> (ExitCode, String, String) {
    let report = doctor_report();
    let validation = rz0_diagnostics_contract::validate_diagnostic_report(&report);
    if !validation.valid {
        return (
            ExitCode::Usage,
            String::new(),
            "foundation diagnostics failed internal validation\n".to_string(),
        );
    }
    match args {
        [] => (
            ExitCode::Ok,
            rz0_diagnostics_contract::diagnostic_text(&report),
            String::new(),
        ),
        [format, value] if format == "--format" && value == "json" => {
            match rz0_diagnostics_contract::diagnostic_json(&report) {
                Ok(json) => (ExitCode::Ok, json, String::new()),
                Err(_) => (
                    ExitCode::Usage,
                    String::new(),
                    "failed to serialize foundation diagnostics\n".to_string(),
                ),
            }
        }
        _ => (
            ExitCode::Usage,
            String::new(),
            format!(
                "unsupported doctor option\n\nUsage: {} doctor [--format json]\n",
                brand::COMMAND
            ),
        ),
    }
}

fn unknown_command(command: &str) -> (ExitCode, String, String) {
    (
        ExitCode::Usage,
        String::new(),
        format!(
            "unknown command '{command}'\n\nRun '{} help' for safe foundation commands.\n",
            brand::COMMAND
        ),
    )
}

fn scan_command(args: &[String]) -> (ExitCode, String, String) {
    let Some(first) = args.first() else {
        return scan_usage_error();
    };
    if first != "--dry-run" {
        return scan_usage_error();
    }
    let mut format = ScanOutputFormat::Text;
    let mut include_raw_paths = false;
    let mut index = 1usize;
    while index < args.len() {
        match args[index].as_str() {
            "--include-raw-paths" => include_raw_paths = true,
            "--format" if args.get(index + 1).is_some_and(|value| value == "json") => {
                format = ScanOutputFormat::Json;
                index += 1;
            }
            _ => return scan_usage_error(),
        }
        index += 1;
    }

    let report = match inventory::live_report(!include_raw_paths) {
        Ok(report) => report,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("local inventory failed closed: {error}\n"),
            );
        }
    };
    match format {
        ScanOutputFormat::Text => (
            ExitCode::Ok,
            inventory::contract_text(&report),
            String::new(),
        ),
        ScanOutputFormat::Json => match inventory::contract_json(&report) {
            Ok(json) => (ExitCode::Ok, json, String::new()),
            Err(err) => (ExitCode::Usage, String::new(), err),
        },
    }
}

fn scan_usage_error() -> (ExitCode, String, String) {
    (
        ExitCode::Usage,
        String::new(),
        format!(
            "scan is report-only and requires dry-run mode\n\nUsage: {} scan --dry-run [--include-raw-paths] [--format json]\n",
            brand::COMMAND
        ),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScanOutputFormat {
    Text,
    Json,
}
