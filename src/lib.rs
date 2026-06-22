use std::env;

pub mod brand;
pub mod module_cli;
pub mod module_manifest;
pub mod module_registry;
pub mod module_validation;

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
        Some("scan") => scan_command(&args[1..]),
        Some(command) => unknown_command(command),
    }
}

pub fn modules_text() -> String {
    module_cli::modules_text()
}

pub fn modules_json() -> Result<String, String> {
    module_cli::modules_json()
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
        "{title} — {subtitle}\n\nUsage:\n  {cmd} --version\n  {cmd} doctor\n  {cmd} modules [--format json]\n  {cmd} modules --from <dir> [--format json]\n  {cmd} modules validate <manifest.json> [--format json]\n  {cmd} scan --dry-run\n\nFoundation safety posture:\n  {safety}\n\nThe core validates local manifests and lists installed modules. It never executes module code or fetches remote modules.\n",
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
        "{title} doctor\n\nstatus: phase-1 bootstrap\ncommand: {cmd}\nversion: {version}\nos: {os}\narch: {arch}\ncurrent_dir: {current_dir}\nsafety: {safety}\nmutation_capability: disabled\ncloudflare_automation: not configured\ngithub_actions: not configured\n",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_includes_brand_and_command() {
        let (code, out, err) = run(["--version"]);
        assert_eq!(code, ExitCode::Ok);
        assert!(err.is_empty());
        assert!(out.contains("runtime.zero"));
        assert!(out.contains("rz0"));
    }

    #[test]
    fn doctor_is_read_only_bootstrap_diagnostic() {
        let (code, out, err) = run(["doctor"]);
        assert_eq!(code, ExitCode::Ok);
        assert!(err.is_empty());
        assert!(out.contains("mutation_capability: disabled"));
        assert!(out.contains("github_actions: not configured"));
    }

    #[test]
    fn modules_show_planned_leftover_scanner() {
        let (code, out, err) = run(["modules"]);
        assert_eq!(code, ExitCode::Ok);
        assert!(err.is_empty());
        assert!(out.contains("installed modules:\n  none"));
        assert!(out.contains("first-party.leftovers"));
        assert!(out.contains("planned"));
    }

    #[test]
    fn modules_json_shows_empty_installed_registry() {
        let (code, out, err) = run(["modules", "--format", "json"]);
        assert_eq!(code, ExitCode::Ok);
        assert!(err.is_empty());
        assert!(out.contains("\"schema_version\": 1"));
        assert!(out.contains("\"installed_modules\": []"));
        assert!(out.contains("\"remote_execution_allowed\": false"));
    }

    #[test]
    fn modules_reject_unknown_options() {
        let (code, out, err) = run(["modules", "--install"]);
        assert_eq!(code, ExitCode::Usage);
        assert!(out.is_empty());
        assert!(err.contains("unsupported modules option"));
    }

    #[test]
    fn modules_validate_rejects_missing_manifest() {
        let (code, out, err) = run(["modules", "validate", "missing-rz0-module.json"]);
        assert_eq!(code, ExitCode::Usage);
        assert!(err.is_empty());
        assert!(out.contains("status: invalid"));
    }

    #[test]
    fn scan_requires_dry_run() {
        let (code, out, err) = run(["scan"]);
        assert_eq!(code, ExitCode::Usage);
        assert!(out.is_empty());
        assert!(err.contains("--dry-run"));
    }

    #[test]
    fn scan_dry_run_attempts_no_changes() {
        let (code, out, err) = run(["scan", "--dry-run"]);
        assert_eq!(code, ExitCode::Ok);
        assert!(err.is_empty());
        assert!(out.contains("mode: dry-run"));
        assert!(out.contains("no system changes were attempted"));
    }
}
