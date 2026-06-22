use std::env;

pub mod brand;
pub mod dashboard_cli;
pub mod install_receipt;
mod install_receipt_schema;
pub mod installed_registry;
mod installed_registry_path;
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
pub mod tui_dashboard;
pub mod tui_render;
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

    match args.first().map(String::as_str) {
        None => (ExitCode::Ok, help_text(), String::new()),
        Some("--help" | "-h" | "help") => (ExitCode::Ok, help_text(), String::new()),
        Some("--version" | "-V" | "version") => (ExitCode::Ok, version_text(), String::new()),
        Some("doctor") => (ExitCode::Ok, doctor_text(), String::new()),
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
        "{title} — {subtitle}\n\nUsage:\n  {cmd}\n  {cmd} --no-tui\n  {cmd} --json\n  {cmd} --version\n  {cmd} doctor\n  {cmd} modules [--format json]\n  {cmd} modules --from <dir> [--format json]\n  {cmd} modules validate <manifest.json> [--format json]\n  {cmd} modules install --dry-run <package-dir-or-manifest> [--format json]\n  {cmd} store plan [--format json]\n  {cmd} store status [--format json]\n  {cmd} store init --dry-run [--format json]\n  {cmd} store init --yes [--format json]\n  {cmd} scan --dry-run\n\nFoundation safety posture:\n  {safety}\n\nThe core validates local manifests and lists installed modules. It never executes module code or fetches remote modules.\n",
        title = brand::TITLE,
        subtitle = brand::SUBTITLE,
        cmd = brand::COMMAND,
        safety = brand::SAFETY_POSTURE
    )
}

pub fn doctor_text() -> String {
    let current_dir = env::current_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "unavailable".to_string());

    format!(
        "{title} doctor\n\nstatus: phase-1 bootstrap\ncommand: {cmd}\nversion: {version}\nos: {os}\narch: {arch}\ncurrent_dir: {current_dir}\nsafety: {safety}\nmutation_capability: explicit_store_init_only\nmodule_mutation_capability: disabled\ncloudflare_automation: not configured\ngithub_actions: not configured\n",
        title = brand::TITLE,
        cmd = brand::COMMAND,
        version = env!("CARGO_PKG_VERSION"),
        os = env::consts::OS,
        arch = env::consts::ARCH,
        current_dir = current_dir,
        safety = brand::SAFETY_POSTURE
    )
}

fn unknown_command(command: &str) -> (ExitCode, String, String) {
    (
        ExitCode::Usage,
        String::new(),
        format!(
            "unknown command '{command}'\n\nRun '{} help' for safe Phase 1 commands.\n",
            brand::COMMAND
        ),
    )
}

fn scan_command(args: &[String]) -> (ExitCode, String, String) {
    let dry_run = args.iter().any(|arg| arg == "--dry-run");
    let unsupported: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|arg| *arg != "--dry-run")
        .collect();

    if !unsupported.is_empty() {
        return (
            ExitCode::Usage,
            String::new(),
            format!(
                "unsupported scan option(s): {}\n\nPhase 1 only supports '{} scan --dry-run'.\n",
                unsupported.join(", "),
                brand::COMMAND
            ),
        );
    }

    if !dry_run {
        return (
            ExitCode::Usage,
            String::new(),
            format!(
                "scan is report-only in Phase 1 and must be run as '{} scan --dry-run'.\n",
                brand::COMMAND
            ),
        );
    }

    (
        ExitCode::Ok,
        format!(
            "{} scan plan\n\nmode: dry-run\nmutation_capability: disabled\nresult: no system changes were attempted\nnext: platform adapters will add read-only inventory in a later phase\n",
            brand::TITLE
        ),
        String::new(),
    )
}
