pub mod apps;
pub mod brand;
pub mod cache;
pub mod color_mode;
pub mod completions;
pub mod dashboard_cli;
mod exact_quarantine;
pub mod install_receipt;
mod install_receipt_schema;
pub mod installed_registry;
mod installed_registry_path;
pub mod integrity;
pub mod inventory;
pub mod launch_routing;
pub mod leftovers;
pub mod module_cli;
pub mod module_install_plan;
pub mod module_manifest;
pub mod module_registry;
pub mod module_store;
pub mod module_trust_cli;
pub mod module_validation;
pub mod package_integrity;
mod package_integrity_io;
pub use rz0_quarantine as quarantine;
pub mod release_cli;
pub mod report;
pub mod store_cli;
pub mod store_init;
mod store_init_model;
pub mod store_init_text;
pub mod store_plan;
pub mod store_status;
pub mod store_status_text;
pub mod system_monitor;
pub mod toolchain;
pub mod tui_app;
pub mod tui_canvas;
pub mod tui_dashboard;
mod tui_dashboard_labels;
pub mod tui_layout;
pub mod tui_ratatui;
mod tui_ratatui_support;
pub mod tui_render;
mod tui_render_support;
pub mod tui_state;
pub mod tui_theme;
pub mod update_cli;
pub mod update_execution;
pub mod updates;

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
        Some("cache") => cache::cache_command(&args[1..]),
        Some("leftovers") => leftovers::leftovers_command(&args[1..]),
        Some("integrity") => integrity::integrity_command(&args[1..]),
        Some("uninstall") => apps::uninstall_command(&args[1..]),
        Some("completions") => completions::completions_command(&args[1..]),
        Some("modules") => module_cli::modules_command(&args[1..]),
        Some("store") => store_cli::store_command(&args[1..]),
        Some("scan") => scan_command(&args[1..]),
        Some("monitor") => system_monitor::monitor_command(&args[1..]),
        Some("toolchain") => toolchain::toolchain_command(&args[1..]),
        Some("report") => report::report_command(&args[1..]),
        Some("release") => release_cli::release_command(&args[1..]),
        Some("updates") => update_cli::updates_command(&args[1..]),
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
    let mut help = format!(
        "{title} — {subtitle}\n\nUsage:\n  {cmd}\n  {cmd} --tui\n  {cmd} --no-tui\n  {cmd} --json\n  {cmd} --color auto|always|never\n  {cmd} --version\n  {cmd} doctor [--format json]\n  {cmd} apps [--format text|json]\n  {cmd} uninstall plan <installed-software-id> [--executable <manager-path>] [--format text|json]\n  {cmd} completions <bash|zsh|fish|powershell>\n  {cmd} modules [--format text|json]\n  {cmd} modules --from <dir> [--format text|json]\n  {cmd} modules validate <manifest.json> [--format text|json]\n  {cmd} modules install --dry-run <package-dir-or-manifest> [--format text|json]\n  {cmd} store plan [--format json]\n  {cmd} store status [--store-root <path>] [--format json]\n  {cmd} store init --dry-run [--format json]\n  {cmd} store init --yes [--format json]\n  {cmd} scan --dry-run [--include-raw-paths] [--format text|json]\n  {cmd} monitor [--format text|json]\n  {cmd} report [--format text|json]\n  {cmd} updates --dry-run --fixture <updater-evidence.json> [--plan] [--queue] [--format text|json]\n  {cmd} updates --dry-run --manager <id> --manager-output <path> --executable <path> [--plan] [--queue] [--format text|json]\n  {cmd} updates --dry-run --probe --manager <id> --executable <path> --allow-network-read [--plan] [--queue] [--format text|json]
  {cmd} updates --apply --probe --manager <id> --executable <path> --allow-network-read --allow-network-write (--action <id> | --all) [--accept-no-rollback] [--challenge-issued-unix-seconds <unix-seconds>] [--confirm <phrase>]\n  {cmd} updates --recovery-status --transaction <id> [--format text|json]\n  {cmd} updates --recovery-complete --transaction <id> [--challenge-issued-unix-seconds <unix-seconds>] [--confirm <phrase>] [--format text|json]\n\nFoundation safety posture:\n  {safety}\n\nThe core includes bounded local inventory, a native system monitor, a privacy-reviewed summary report, local manifest validation, and installed-module listing. Mutating updates require explicit apply mode, plan-sealed manager identity, a reviewed identity-to-spawn binding, network-write approval, a short-lived plan-bound confirmation, durable external-effect transaction evidence, and fresh post-action verification. Recovery status is read-only; recovery completion may append only the exact final local journal commit for a verified receipt and never reruns a manager. Uninstall and module execution remain separately gated.\n",
        title = brand::TITLE,
        subtitle = brand::SUBTITLE,
        cmd = brand::COMMAND,
        safety = brand::SAFETY_POSTURE
    );
    let toolchain_usage = format!("  {} toolchain [--format text|json]\n", brand::COMMAND);
    if let Some(index) = help.find(&format!("  {} report", brand::COMMAND)) {
        help.insert_str(index, &toolchain_usage);
    }
    let cache_usage = format!(
        "  {} cache --dry-run [--format text|json] [--fixture <cache-input.json>]\n  {} cache --dry-run --plan --path <absolute-cache-file> [--format text|json]\n  {} cache --apply --path <absolute-cache-file> [--challenge-issued-unix-seconds <seconds>] [--confirm <exact-phrase>] [--format text|json]\n",
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND,
    );
    if let Some(index) = help.find(&format!("  {} uninstall", brand::COMMAND)) {
        help.insert_str(index, &cache_usage);
    }
    let leftovers_usage = format!(
        "  {} leftovers --dry-run [--format text|json] [--fixture <leftover-input.json>]\n  {} leftovers --dry-run --plan --path <absolute-module-file> [--format text|json]\n  {} leftovers --apply --path <absolute-module-file> [--challenge-issued-unix-seconds <seconds>] [--confirm <exact-phrase>] [--format text|json]\n",
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND,
    );
    if let Some(index) = help.find(&format!("  {} uninstall", brand::COMMAND)) {
        help.insert_str(index, &leftovers_usage);
    }
    let integrity_usage = format!(
        "  {} integrity --dry-run --fixture <integrity-input.json> [--format text|json]\n  {} integrity --dry-run --path <absolute-file> --sha256 <digest> [--format text|json]\n",
        brand::COMMAND,
        brand::COMMAND
    );
    if let Some(index) = help.find(&format!("  {} uninstall", brand::COMMAND)) {
        help.insert_str(index, &integrity_usage);
    }
    let lifecycle_usage = format!(
        "  {} modules lifecycle-plan <operation> --dry-run --module-id <id> --from-state <state> --to-state <state> [--from-version <version>] [--to-version <version>] [--format text|json]\n",
        brand::COMMAND
    );
    if let Some(index) = help.find("\n\nFoundation safety posture:") {
        help.insert_str(index, &format!("\n{lifecycle_usage}"));
    }
    let module_trust_usage = format!(
        "  {} modules trust verify --manifest <manifest.json> --signature <envelope.json> --trusted-test-key <key.json> [--format text|json]\n",
        brand::COMMAND
    );
    if let Some(index) = help.find("\n\nFoundation safety posture:") {
        help.insert_str(index, &format!("\n{module_trust_usage}"));
    }
    let release_usage = format!(
        "  {} release status --assessment <assessment.json> [--format text|json]\n",
        brand::COMMAND
    );
    if let Some(index) = help.find("\n\nFoundation safety posture:") {
        help.insert_str(index, &format!("\n{release_usage}"));
    }
    let provider_usage = format!(
        "  {} updates --dry-run --all-providers --allow-network-read [--plan] [--queue] [--format text|json]\n  {} updates --apply --all-providers --allow-network-read --allow-network-write [--accept-no-rollback]\n  {} updates --apply --all-providers --allow-network-read --allow-network-write --action <exact-action-id> --accept-no-rollback\n",
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND,
    );
    if let Some(index) = help.find("\n\nFoundation safety posture:") {
        help.insert_str(index, &format!("\n{provider_usage}"));
    } else {
        help.push_str(&provider_usage);
    }
    help
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
        [json] if json == "--json" => match rz0_diagnostics_contract::diagnostic_json(&report) {
            Ok(json) => (ExitCode::Ok, json, String::new()),
            Err(_) => (
                ExitCode::Usage,
                String::new(),
                "failed to serialize foundation diagnostics\n".to_string(),
            ),
        },
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
                "unsupported doctor option\n\nUsage: {} doctor [--format json|--json]\n",
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
    if matches!(args, [help] if matches!(help.as_str(), "--help" | "-h" | "help")) {
        return (ExitCode::Ok, scan_usage(), String::new());
    }
    let mut dry_run = false;
    let mut format = ScanOutputFormat::Text;
    let mut include_raw_paths = false;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--dry-run" if !dry_run => dry_run = true,
            "--dry-run" => return scan_usage_error(),
            "--include-raw-paths" => include_raw_paths = true,
            "--json" => format = ScanOutputFormat::Json,
            "--format" => {
                let Some(value) = args.get(index + 1).map(String::as_str) else {
                    return scan_usage_error();
                };
                format = match value {
                    "text" => ScanOutputFormat::Text,
                    "json" => ScanOutputFormat::Json,
                    _ => return scan_usage_error(),
                };
                index += 1;
            }
            _ => return scan_usage_error(),
        }
        index += 1;
    }
    if !dry_run {
        return scan_usage_error();
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

fn scan_usage() -> String {
    format!(
        "Usage: {} scan --dry-run [--include-raw-paths] [--format text|json]\n\nReports bounded local evidence without writing system state.\n",
        brand::COMMAND
    )
}

fn scan_usage_error() -> (ExitCode, String, String) {
    (
        ExitCode::Usage,
        String::new(),
        format!(
            "scan is report-only and requires dry-run mode\n\n{}",
            scan_usage()
        ),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScanOutputFormat {
    Text,
    Json,
}
