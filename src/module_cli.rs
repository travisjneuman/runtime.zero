use std::fmt::Write as FmtWrite;
use std::path::Path;

use crate::{
    ExitCode, brand, module_install_plan, module_manifest, module_registry, module_validation,
};
use rz0_module_lifecycle::{ModuleLifecycleOperation, ModuleLifecycleState, module_lifecycle_plan};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

enum ModulesAction {
    Help,
    List {
        format: OutputFormat,
        from: Option<String>,
    },
    Validate {
        path: String,
        format: OutputFormat,
    },
    InstallDryRun {
        path: String,
        format: OutputFormat,
    },
    LifecyclePlan {
        operation: ModuleLifecycleOperation,
        module_id: String,
        from_state: ModuleLifecycleState,
        to_state: ModuleLifecycleState,
        from_version: Option<String>,
        to_version: Option<String>,
        transition_id: Option<String>,
        format: OutputFormat,
    },
}

struct LifecyclePlanRenderRequest {
    operation: ModuleLifecycleOperation,
    module_id: String,
    from_state: ModuleLifecycleState,
    to_state: ModuleLifecycleState,
    from_version: Option<String>,
    to_version: Option<String>,
    transition_id: Option<String>,
}

pub fn modules_command(args: &[String]) -> (ExitCode, String, String) {
    match parse_modules_args(args) {
        Ok(ModulesAction::Help) => (ExitCode::Ok, modules_usage(), String::new()),
        Ok(ModulesAction::List { format, from }) => render_modules(format, from.as_deref()),
        Ok(ModulesAction::Validate { path, format }) => render_validation(format, &path),
        Ok(ModulesAction::InstallDryRun { path, format }) => render_install_plan(format, &path),
        Ok(ModulesAction::LifecyclePlan {
            operation,
            module_id,
            from_state,
            to_state,
            from_version,
            to_version,
            transition_id,
            format,
        }) => render_lifecycle_plan(
            format,
            LifecyclePlanRenderRequest {
                operation,
                module_id,
                from_state,
                to_state,
                from_version,
                to_version,
                transition_id,
            },
        ),
        Err(err) => (ExitCode::Usage, String::new(), err),
    }
}

pub fn modules_text() -> String {
    modules_text_from(None)
}

pub fn modules_json() -> Result<String, String> {
    modules_json_from(None)
}

fn parse_modules_args(args: &[String]) -> Result<ModulesAction, String> {
    match args.first().map(String::as_str) {
        Some(flag) if matches!(flag, "--help" | "-h" | "help") && args.len() == 1 => {
            Ok(ModulesAction::Help)
        }
        Some("validate") => parse_validate_args(&args[1..]),
        Some("install") => parse_install_args(&args[1..]),
        Some("lifecycle-plan") => parse_lifecycle_plan_args(&args[1..]),
        _ => parse_list_args(args),
    }
}

fn parse_list_args(args: &[String]) -> Result<ModulesAction, String> {
    let mut format = OutputFormat::Text;
    let mut from = None;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => format = OutputFormat::Json,
            "--format" => {
                format = parse_format(args, &mut index)?;
            }
            "--from" => {
                let Some(path) = args.get(index + 1) else {
                    return Err(usage_error(args));
                };
                if from.replace(path.clone()).is_some() {
                    return Err("module source directory was provided more than once".to_string());
                }
                index += 1;
            }
            _ => return Err(usage_error(args)),
        }
        index += 1;
    }
    Ok(list_action(format, from.as_ref()))
}

fn parse_validate_args(args: &[String]) -> Result<ModulesAction, String> {
    let mut format = OutputFormat::Text;
    let mut path = None;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => format = OutputFormat::Json,
            "--format" => format = parse_format(args, &mut index)?,
            value if path.is_none() => path = Some(value.to_string()),
            _ => return Err(usage_error(args)),
        }
        index += 1;
    }
    let Some(path) = path else {
        return Err(usage_error(args));
    };
    Ok(validate_action(&path, format))
}

fn parse_install_args(args: &[String]) -> Result<ModulesAction, String> {
    let mut format = OutputFormat::Text;
    let mut dry_run = false;
    let mut path = None;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--dry-run" if !dry_run => dry_run = true,
            "--dry-run" => return Err(install_dry_run_usage()),
            "--json" => format = OutputFormat::Json,
            "--format" => format = parse_format(args, &mut index)?,
            value if path.is_none() => path = Some(value.to_string()),
            _ => return Err(install_dry_run_usage()),
        }
        index += 1;
    }
    if !dry_run {
        return Err(install_dry_run_usage());
    }
    let Some(path) = path else {
        return Err(usage_error(args));
    };
    Ok(install_dry_run_action(&path, format))
}

fn parse_lifecycle_plan_args(args: &[String]) -> Result<ModulesAction, String> {
    let mut operation = None;
    let mut module_id = None;
    let mut from_state = None;
    let mut to_state = None;
    let mut from_version = None;
    let mut to_version = None;
    let mut transition_id = None;
    let mut format = OutputFormat::Text;
    let mut dry_run = false;
    let mut index = 0usize;

    while index < args.len() {
        match args[index].as_str() {
            "--dry-run" if !dry_run => dry_run = true,
            "--dry-run" => return Err(lifecycle_plan_usage()),
            "--module-id" => set_option(&mut module_id, args, &mut index, "module id")?,
            "--from-state" => set_option(&mut from_state, args, &mut index, "from state")?,
            "--to-state" => set_option(&mut to_state, args, &mut index, "to state")?,
            "--from-version" => set_option(&mut from_version, args, &mut index, "from version")?,
            "--to-version" => set_option(&mut to_version, args, &mut index, "to version")?,
            "--transition-id" => set_option(&mut transition_id, args, &mut index, "transition id")?,
            "--json" => format = OutputFormat::Json,
            "--format" => format = parse_format(args, &mut index)?,
            value if operation.is_none() => operation = Some(parse_operation(value)?),
            _ => return Err(lifecycle_plan_usage()),
        }
        index += 1;
    }

    if !dry_run {
        return Err(lifecycle_plan_usage());
    }
    let Some(operation) = operation else {
        return Err(lifecycle_plan_usage());
    };
    let Some(module_id) = module_id else {
        return Err(lifecycle_plan_usage());
    };
    let Some(from_state) = from_state.and_then(|value| parse_state(&value).ok()) else {
        return Err(lifecycle_plan_usage());
    };
    let Some(to_state) = to_state.and_then(|value| parse_state(&value).ok()) else {
        return Err(lifecycle_plan_usage());
    };

    Ok(ModulesAction::LifecyclePlan {
        operation,
        module_id: module_id.clone(),
        from_state,
        to_state,
        from_version,
        to_version,
        transition_id,
        format,
    })
}

fn set_option(
    slot: &mut Option<String>,
    args: &[String],
    index: &mut usize,
    label: &str,
) -> Result<(), String> {
    let Some(value) = args.get(*index + 1) else {
        return Err(format!("missing {label}\n\n{}", lifecycle_plan_usage()));
    };
    if slot.replace(value.clone()).is_some() {
        return Err(format!("{label} was provided more than once"));
    }
    *index += 1;
    Ok(())
}

fn parse_operation(value: &str) -> Result<ModuleLifecycleOperation, String> {
    match value {
        "install" => Ok(ModuleLifecycleOperation::Install),
        "activate" => Ok(ModuleLifecycleOperation::Activate),
        "invoke" => Ok(ModuleLifecycleOperation::Invoke),
        "deactivate" => Ok(ModuleLifecycleOperation::Deactivate),
        "repair" => Ok(ModuleLifecycleOperation::Repair),
        "migrate" => Ok(ModuleLifecycleOperation::Migrate),
        "upgrade" => Ok(ModuleLifecycleOperation::Upgrade),
        "uninstall" => Ok(ModuleLifecycleOperation::Uninstall),
        _ => Err(format!(
            "unsupported lifecycle operation '{value}'\n\n{}",
            lifecycle_plan_usage()
        )),
    }
}

fn parse_state(value: &str) -> Result<ModuleLifecycleState, String> {
    match value {
        "absent" => Ok(ModuleLifecycleState::Absent),
        "staged" => Ok(ModuleLifecycleState::Staged),
        "installed_inactive" => Ok(ModuleLifecycleState::InstalledInactive),
        "active" => Ok(ModuleLifecycleState::Active),
        "degraded" => Ok(ModuleLifecycleState::Degraded),
        "quarantined" => Ok(ModuleLifecycleState::Quarantined),
        _ => Err(format!("unsupported lifecycle state '{value}'")),
    }
}

fn parse_format(args: &[String], index: &mut usize) -> Result<OutputFormat, String> {
    let Some(value) = args.get(*index + 1).map(String::as_str) else {
        return Err(usage_error(args));
    };
    *index += 1;
    match value {
        "text" => Ok(OutputFormat::Text),
        "json" => Ok(OutputFormat::Json),
        _ => Err(usage_error(args)),
    }
}

fn install_dry_run_usage() -> String {
    format!(
        "module install planning is dry-run only\n\nUsage: {} modules install --dry-run <package-dir-or-manifest> [--format text|json]\n",
        brand::COMMAND
    )
}

fn lifecycle_plan_usage() -> String {
    format!(
        "module lifecycle planning is dry-run only\n\nUsage: {} modules lifecycle-plan <install|activate|invoke|deactivate|repair|migrate|upgrade|uninstall> --dry-run --module-id <id> --from-state <state> --to-state <state> [--from-version <version>] [--to-version <version>] [--transition-id <id>] [--format text|json]\n\nStates: absent, staged, installed_inactive, active, degraded, quarantined\nSafety: this command creates a digest-bound, non-authorizing plan; it does not write, execute, activate, disable, or uninstall a module.\n",
        brand::COMMAND
    )
}

fn list_action(format: OutputFormat, from: Option<&String>) -> ModulesAction {
    ModulesAction::List {
        format,
        from: from.cloned(),
    }
}

fn validate_action(path: &str, format: OutputFormat) -> ModulesAction {
    ModulesAction::Validate {
        path: path.to_string(),
        format,
    }
}

fn install_dry_run_action(path: &str, format: OutputFormat) -> ModulesAction {
    ModulesAction::InstallDryRun {
        path: path.to_string(),
        format,
    }
}

fn render_modules(format: OutputFormat, from: Option<&str>) -> (ExitCode, String, String) {
    match format {
        OutputFormat::Text => (ExitCode::Ok, modules_text_from(from), String::new()),
        OutputFormat::Json => match modules_json_from(from) {
            Ok(json) => (ExitCode::Ok, json, String::new()),
            Err(err) => (ExitCode::Usage, String::new(), err),
        },
    }
}

fn render_validation(format: OutputFormat, path: &str) -> (ExitCode, String, String) {
    let report = module_validation::load_manifest_file(Path::new(path));
    let code = if report.valid {
        ExitCode::Ok
    } else {
        ExitCode::Usage
    };
    match format {
        OutputFormat::Text => (code, validation_text(&report), String::new()),
        OutputFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(json) => (code, format!("{json}\n"), String::new()),
            Err(err) => (ExitCode::Usage, String::new(), err.to_string()),
        },
    }
}

fn render_install_plan(format: OutputFormat, path: &str) -> (ExitCode, String, String) {
    let report = module_install_plan::plan_module_install_dry_run(Path::new(path));
    let code = if report.valid {
        ExitCode::Ok
    } else {
        ExitCode::Usage
    };
    match format {
        OutputFormat::Text => (code, install_plan_text(&report), String::new()),
        OutputFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(json) => (code, format!("{json}\n"), String::new()),
            Err(err) => (ExitCode::Usage, String::new(), err.to_string()),
        },
    }
}

fn render_lifecycle_plan(
    format: OutputFormat,
    request: LifecyclePlanRenderRequest,
) -> (ExitCode, String, String) {
    let LifecyclePlanRenderRequest {
        operation,
        module_id,
        from_state,
        to_state,
        from_version,
        to_version,
        transition_id,
    } = request;
    let transition_id = transition_id.unwrap_or_else(|| {
        format!(
            "module-lifecycle-{}-{}",
            operation_label(operation),
            module_id
        )
    });
    let plan = module_lifecycle_plan(
        transition_id,
        module_id,
        operation,
        from_state,
        to_state,
        from_version,
        to_version,
    );
    match plan {
        Ok(plan) => match format {
            OutputFormat::Text => (ExitCode::Ok, lifecycle_plan_text(&plan), String::new()),
            OutputFormat::Json => match serde_json::to_string_pretty(&plan) {
                Ok(json) => (ExitCode::Ok, format!("{json}\n"), String::new()),
                Err(err) => (ExitCode::Usage, String::new(), err.to_string()),
            },
        },
        Err(validation) => {
            let message = match format {
                OutputFormat::Text => {
                    let mut out = format!(
                        "{} module lifecycle plan\n\nstatus: invalid\n",
                        brand::TITLE
                    );
                    for error in validation.errors {
                        let _ = writeln!(out, "error: {error}");
                    }
                    out
                }
                OutputFormat::Json => {
                    serde_json::json!({
                        "schema_version": rz0_module_lifecycle::MODULE_LIFECYCLE_SCHEMA_VERSION,
                        "contract": rz0_module_lifecycle::MODULE_LIFECYCLE_CONTRACT,
                        "valid": false,
                        "errors": validation.errors,
                        "dry_run": true,
                        "writes_attempted": false,
                        "product_execution_authorized": false,
                    })
                    .to_string()
                        + "\n"
                }
            };
            (ExitCode::Usage, message, String::new())
        }
    }
}

fn lifecycle_plan_text(plan: &rz0_module_lifecycle::ModuleLifecyclePlan) -> String {
    let mut out = format!("{} module lifecycle plan\n\n", brand::TITLE);
    let _ = writeln!(out, "status: valid");
    let _ = writeln!(out, "contract: {}", plan.contract);
    let _ = writeln!(out, "transition_id: {}", plan.transition_id);
    let _ = writeln!(out, "module_id: {}", plan.module_id);
    let _ = writeln!(out, "operation: {}", operation_label(plan.operation));
    let _ = writeln!(out, "from_state: {}", state_label(plan.from_state));
    let _ = writeln!(out, "to_state: {}", state_label(plan.to_state));
    let _ = writeln!(
        out,
        "from_version: {}",
        optional_value(plan.from_version.as_deref())
    );
    let _ = writeln!(
        out,
        "to_version: {}",
        optional_value(plan.to_version.as_deref())
    );
    let _ = writeln!(
        out,
        "required_gates: {}",
        plan.required_gates
            .iter()
            .map(|gate| format!("{gate:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let _ = writeln!(out, "dry_run: {}", plan.dry_run);
    let _ = writeln!(out, "writes_attempted: {}", plan.writes_attempted);
    let _ = writeln!(out, "would_mutate: {}", plan.would_mutate);
    let _ = writeln!(out, "rollback_required: {}", plan.rollback_required);
    let _ = writeln!(
        out,
        "explicit_confirmation_required: {}",
        plan.explicit_confirmation_required
    );
    let _ = writeln!(
        out,
        "product_execution_authorized: {}",
        plan.product_execution_authorized
    );
    let _ = writeln!(out, "plan_sha256: {}", plan.plan_sha256);
    let _ = writeln!(
        out,
        "safety: dry-run plan only; no lifecycle action was executed"
    );
    out
}

fn optional_value(value: Option<&str>) -> &str {
    value.unwrap_or("none")
}

fn operation_label(operation: ModuleLifecycleOperation) -> &'static str {
    match operation {
        ModuleLifecycleOperation::Install => "install",
        ModuleLifecycleOperation::Activate => "activate",
        ModuleLifecycleOperation::Invoke => "invoke",
        ModuleLifecycleOperation::Deactivate => "deactivate",
        ModuleLifecycleOperation::Repair => "repair",
        ModuleLifecycleOperation::Migrate => "migrate",
        ModuleLifecycleOperation::Upgrade => "upgrade",
        ModuleLifecycleOperation::Uninstall => "uninstall",
    }
}

fn state_label(state: ModuleLifecycleState) -> &'static str {
    match state {
        ModuleLifecycleState::Absent => "absent",
        ModuleLifecycleState::Staged => "staged",
        ModuleLifecycleState::InstalledInactive => "installed_inactive",
        ModuleLifecycleState::Active => "active",
        ModuleLifecycleState::Degraded => "degraded",
        ModuleLifecycleState::Quarantined => "quarantined",
    }
}

fn modules_text_from(from: Option<&str>) -> String {
    let report = registry_report(from);
    let mut out = format!("{} modules\n\n", brand::TITLE);
    write_core(&mut out, &report.core);
    write_installed(&mut out, &report);
    write_planned(&mut out, &report.planned_module_families);
    let _ = writeln!(
        out,
        "\nsafety: optional modules are not bundled, installed, or executed by default"
    );
    out
}

fn write_core(out: &mut String, modules: &[module_manifest::ModuleManifest]) {
    let _ = writeln!(out, "core foundation:");
    for module in modules {
        let _ = writeln!(out, "  {:<16} active   {}", module.id, module.summary);
    }
}

fn write_installed(out: &mut String, report: &module_registry::ModuleRegistryReport) {
    let _ = writeln!(out, "\ninstalled modules:");
    if report.installed_modules.is_empty() {
        let _ = writeln!(out, "  none");
    } else {
        for module in &report.installed_modules {
            let _ = writeln!(out, "  {:<22} installed {}", module.id, module.summary);
        }
    }
    if !report.validation_reports.is_empty() {
        write_validation_summary(out, &report.validation_reports);
    }
}

fn write_validation_summary(
    out: &mut String,
    reports: &[module_validation::ManifestValidationReport],
) {
    let _ = writeln!(out, "\nvalidation:");
    for report in reports {
        let status = if report.valid { "valid" } else { "invalid" };
        let _ = writeln!(out, "  {:<8} {}", status, report.path);
    }
}

fn write_planned(out: &mut String, modules: &[module_manifest::ModuleManifest]) {
    let _ = writeln!(out, "\nplanned first-party module families:");
    for module in modules {
        let _ = writeln!(out, "  {:<22} planned  {}", module.id, module.summary);
    }
}

fn modules_json_from(from: Option<&str>) -> Result<String, String> {
    serde_json::to_string_pretty(&registry_report(from))
        .map(|json| format!("{json}\n"))
        .map_err(|err| format!("failed to render module registry JSON: {err}\n"))
}

fn registry_report(from: Option<&str>) -> module_registry::ModuleRegistryReport {
    match from {
        Some(path) => module_registry::ModuleRegistryReport::from_directory(Path::new(path)),
        None => module_registry::ModuleRegistryReport::empty_installed(),
    }
}

fn validation_text(report: &module_validation::ManifestValidationReport) -> String {
    let status = if report.valid { "valid" } else { "invalid" };
    let mut out = format!("{} module manifest validation\n\n", brand::TITLE);
    let _ = writeln!(out, "path: {}", report.path);
    let _ = writeln!(out, "status: {status}");
    for error in &report.errors {
        let _ = writeln!(out, "error: {error}");
    }
    for warning in &report.warnings {
        let _ = writeln!(out, "warning: {warning}");
    }
    out
}

fn install_plan_text(report: &module_install_plan::ModuleInstallPlanReport) -> String {
    let status = if report.valid { "valid" } else { "invalid" };
    let mut out = format!("{} module install dry-run\n\n", brand::TITLE);
    let _ = writeln!(out, "input: {}", report.input_path);
    let _ = writeln!(out, "manifest: {}", report.manifest_path);
    let _ = writeln!(out, "package_root: {}", report.package_root);
    let _ = writeln!(out, "status: {status}");
    let _ = writeln!(out, "dry_run: true");
    let _ = writeln!(out, "writes_attempted: no");
    if let Some(target) = &report.proposed_module_dir {
        let _ = writeln!(out, "proposed_module_dir: {target}");
    }
    for action in &report.planned_actions {
        let _ = writeln!(
            out,
            "plan: {} -> {}",
            install_action_label(action.action),
            action.target
        );
    }
    for error in &report.errors {
        let _ = writeln!(out, "error: {error}");
    }
    for warning in &report.validation.warnings {
        let _ = writeln!(out, "warning: {warning}");
    }
    let _ = writeln!(out, "safety: {}", report.safety_note);
    out
}

fn install_action_label(action: module_install_plan::PlannedInstallActionKind) -> &'static str {
    match action {
        module_install_plan::PlannedInstallActionKind::CreateModuleDirectory => {
            "create_module_directory"
        }
        module_install_plan::PlannedInstallActionKind::CopyPackageFile => "copy_package_file",
        module_install_plan::PlannedInstallActionKind::RecordInstalledManifest => {
            "record_installed_manifest"
        }
    }
}

fn usage_error(args: &[String]) -> String {
    format!(
        "unsupported modules option(s): {}\n\n{}",
        args.join(", "),
        modules_usage()
    )
}

fn modules_usage() -> String {
    format!(
        "Usage: {} modules [--from <dir>] [--format text|json]\n       {} modules validate <manifest.json> [--format text|json]\n       {} modules install --dry-run <package-dir-or-manifest> [--format text|json]\n       {} modules lifecycle-plan <operation> --dry-run --module-id <id> --from-state <state> --to-state <state> [--from-version <version>] [--to-version <version>] [--format text|json]\n\nSafety: module install and lifecycle planning are dry-run only; modules are not executed or fetched.\n",
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND
    )
}
