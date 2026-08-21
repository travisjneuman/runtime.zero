use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::{Path, PathBuf};

use rz0_action_plan::{
    ActionCapability, ActionDisposition, ActionKind, ActionPlan, ActionRisk, ActionSource,
    PlanAction, RollbackPlan, WriteKind, WriteSetEntry, validate_action_plan,
};
use rz0_quarantine::{QuarantineRecord, validate_quarantine_record};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    ExitCode, brand,
    exact_quarantine::{
        build_exact_quarantine_challenge, execute_exact_quarantine,
        render_exact_filesystem_challenge, unix_seconds, validate_exact_quarantine_confirmation,
    },
    module_store::{ModuleStorePlan, module_store_plan},
};

const RESTORE_REVIEW_CONTRACT: &str = "restore_review";
const MAX_RECORD_BYTES: u64 = rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct RestoreReview {
    schema_version: u16,
    contract: &'static str,
    read_only: bool,
    writes_attempted: bool,
    plan_id: String,
    action_plan: ActionPlan,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone)]
struct Options {
    dry_run: bool,
    plan_id: String,
    confirm: Option<String>,
    challenge_issued_unix_seconds: Option<u64>,
    format: OutputFormat,
}

pub fn restore_command(args: &[String]) -> (ExitCode, String, String) {
    if matches!(args, [help] if matches!(help.as_str(), "--help" | "-h" | "help")) {
        return (ExitCode::Ok, usage(), String::new());
    }
    let options = match parse_args(args) {
        Ok(options) => options,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("{error}\n\n{}", usage()),
            );
        }
    };
    let store = module_store_plan(None, None, "restore exact quarantine");
    let action_plan = match exact_restore_plan(&store, &options.plan_id) {
        Ok(plan) => plan,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("restore action plan failed closed: {error}\n"),
            );
        }
    };
    let issued = options
        .challenge_issued_unix_seconds
        .unwrap_or_else(unix_seconds);
    let challenge = match build_exact_quarantine_challenge(&action_plan, issued) {
        Ok(challenge) => challenge,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("restore confirmation challenge failed closed: {error}\n"),
            );
        }
    };
    if options.dry_run {
        let review = RestoreReview {
            schema_version: 1,
            contract: RESTORE_REVIEW_CONTRACT,
            read_only: true,
            writes_attempted: false,
            plan_id: action_plan.plan_id.clone(),
            action_plan,
            warnings: vec![
                "restore planning reads one existing validated quarantine record; no file was moved"
                    .to_string(),
                "restore refuses occupied destinations and never deletes or recurses".to_string(),
            ],
        };
        return render_review(&review, options.format);
    }
    let Some(phrase) = options.confirm.as_deref() else {
        return (
            ExitCode::Ok,
            render_exact_filesystem_challenge(
                &challenge,
                &action_plan.actions[0].action_id,
                "restore",
                options.format == OutputFormat::Json,
            ),
            String::new(),
        );
    };
    let response = match validate_exact_quarantine_confirmation(&challenge, phrase, unix_seconds())
    {
        Ok(response) => response,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("restore confirmation rejected: {error}\n"),
            );
        }
    };
    let effect =
        match execute_exact_quarantine(&store, &action_plan, &challenge, &response, unix_seconds())
        {
            Ok(effect) => effect,
            Err(error) => {
                return (
                    ExitCode::Usage,
                    String::new(),
                    format!(
                        "restore filesystem effect failed closed [{:?}]: {error}\n",
                        error.code
                    ),
                );
            }
        };
    render_effect(&effect, options.format)
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut dry_run = false;
    let mut apply = false;
    let mut plan_id = None;
    let mut confirm = None;
    let mut challenge_issued_unix_seconds = None;
    let mut format = OutputFormat::Text;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--dry-run" if !dry_run => dry_run = true,
            "--dry-run" => return Err("restore --dry-run was provided more than once".to_string()),
            "--apply" if !apply => apply = true,
            "--apply" => return Err("restore --apply was provided more than once".to_string()),
            "--plan-id" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(
                        "restore --plan-id requires one exact quarantine plan id".to_string()
                    );
                };
                if plan_id.replace(value.clone()).is_some() {
                    return Err("restore --plan-id was provided more than once".to_string());
                }
                index += 1;
            }
            "--confirm" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("restore --confirm requires the exact challenge phrase".to_string());
                };
                if confirm.replace(value.clone()).is_some() {
                    return Err("restore --confirm was provided more than once".to_string());
                }
                index += 1;
            }
            "--challenge-issued-unix-seconds" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(
                        "restore --challenge-issued-unix-seconds requires an integer".to_string(),
                    );
                };
                if challenge_issued_unix_seconds.is_some() {
                    return Err(
                        "restore --challenge-issued-unix-seconds was provided more than once"
                            .to_string(),
                    );
                }
                challenge_issued_unix_seconds = Some(value.parse().map_err(|_| {
                    "restore --challenge-issued-unix-seconds must be an integer".to_string()
                })?);
                index += 1;
            }
            "--format" => {
                let Some(value) = args.get(index + 1).map(String::as_str) else {
                    return Err("restore --format requires text or json".to_string());
                };
                format = match value {
                    "text" => OutputFormat::Text,
                    "json" => OutputFormat::Json,
                    _ => return Err("restore --format requires text or json".to_string()),
                };
                index += 1;
            }
            "--json" => format = OutputFormat::Json,
            value => return Err(format!("unsupported restore option '{value}'")),
        }
        index += 1;
    }
    let plan_id = plan_id.ok_or_else(|| "restore requires --plan-id".to_string())?;
    if !rz0_validation_contract::valid_dotted_id(&plan_id, 100) || !plan_id.starts_with("rz0plan-")
    {
        return Err(
            "restore --plan-id must be an exact runtime.zero quarantine plan id".to_string(),
        );
    }
    if dry_run == apply {
        return Err("restore requires exactly one of --dry-run or --apply".to_string());
    }
    if !apply && (confirm.is_some() || challenge_issued_unix_seconds.is_some()) {
        return Err("restore confirmation options require --apply".to_string());
    }
    Ok(Options {
        dry_run,
        plan_id,
        confirm,
        challenge_issued_unix_seconds,
        format,
    })
}

fn exact_restore_plan(store: &ModuleStorePlan, plan_id: &str) -> Result<ActionPlan, String> {
    let record = read_quarantine_record(store, plan_id)?;
    let module_id = module_id_for_original_path(&record.original_path)?;
    build_restore_action_plan(module_id, &record)
}

fn read_quarantine_record(
    store: &ModuleStorePlan,
    plan_id: &str,
) -> Result<QuarantineRecord, String> {
    let root = PathBuf::from(&store.quarantine_root);
    let relative = Path::new(plan_id).join("quarantine.json");
    let mut current = root.clone();
    for component in relative.components() {
        let std::path::Component::Normal(name) = component else {
            return Err("restore quarantine record path is unsafe".to_string());
        };
        current.push(name);
        let metadata = fs::symlink_metadata(&current)
            .map_err(|error| format!("inspect restore quarantine record: {error}"))?;
        if metadata.file_type().is_symlink() {
            return Err("restore quarantine record refuses symlinked components".to_string());
        }
    }
    let metadata = fs::symlink_metadata(&current)
        .map_err(|error| format!("inspect restore quarantine record: {error}"))?;
    if !metadata.is_file() || metadata.len() > MAX_RECORD_BYTES {
        return Err("restore quarantine record must be a bounded regular file".to_string());
    }
    let bytes =
        fs::read(&current).map_err(|error| format!("read restore quarantine record: {error}"))?;
    let record: QuarantineRecord = serde_json::from_slice(&bytes)
        .map_err(|error| format!("decode restore quarantine record: {error}"))?;
    if !validate_quarantine_record(&record)
        || record.plan_id != plan_id
        || record.quarantine_path != format!("quarantine/{plan_id}/payload.bin")
    {
        return Err("restore quarantine record failed closed validation".to_string());
    }
    Ok(record)
}

fn module_id_for_original_path(path: &str) -> Result<&'static str, String> {
    if path.starts_with("workspace/cache/") {
        Ok(rz0_module_cache::MODULE_ID)
    } else if path.starts_with("workspace/modules/") {
        Ok(rz0_module_leftovers::MODULE_ID)
    } else {
        Err("restore source is outside the supported exact cache/module namespaces".to_string())
    }
}

fn build_restore_action_plan(
    module_id: &str,
    record: &QuarantineRecord,
) -> Result<ActionPlan, String> {
    let namespace = if module_id == rz0_module_cache::MODULE_ID {
        "workspace/cache/"
    } else if module_id == rz0_module_leftovers::MODULE_ID {
        "workspace/modules/"
    } else {
        return Err("restore module is not a supported first-party exact-file lane".to_string());
    };
    if !record.original_path.starts_with(namespace) {
        return Err("restore record namespace does not match its module".to_string());
    }
    let short = short_digest(record.binding_sha256.as_bytes());
    let suffix = module_id.rsplit('.').next().unwrap_or("module");
    let plan = ActionPlan {
        schema_version: rz0_action_plan::ACTION_PLAN_SCHEMA_VERSION,
        plan_id: format!("rz0restore-{suffix}-{short}"),
        module_id: module_id.to_string(),
        created_at: None,
        expires_at: None,
        dry_run: true,
        writes_attempted: false,
        evidence_contract: rz0_finding_contract::FINDING_CONTRACT.to_string(),
        evidence_report_id: format!("findings:{}", &record.binding_sha256[..24]),
        evidence_sha256: record.binding_sha256.clone(),
        actions: vec![PlanAction {
            action_id: format!("restore-{suffix}-{short}"),
            finding_id: format!("restore.{suffix}.{short}"),
            kind: ActionKind::Restore,
            disposition: ActionDisposition::Planned,
            target: record.original_path.clone(),
            source: Some(ActionSource {
                path: record.quarantine_path.clone(),
                sha256: record.sha256.clone(),
                size_bytes: record.size_bytes,
            }),
            manager: None,
            executable: None,
            executable_identity: None,
            arguments: Vec::new(),
            would_write: false,
            requires_confirmation: true,
            requires_elevation: false,
            network_required: false,
            risk: ActionRisk::Medium,
            capabilities: vec![ActionCapability::RuntimeStateWrite, ActionCapability::RestoreWrite],
            forbidden_path_classes: Vec::new(),
            write_set: vec![WriteSetEntry {
                path: record.original_path.clone(),
                kind: WriteKind::RestoredPayload,
            }],
            rollback: RollbackPlan {
                supported: true,
                quarantine_required: false,
                description:
                    "A restored file remains protected by a fresh exact quarantine plan; no overwrite is permitted."
                        .to_string(),
            },
        }],
        warnings: vec![
            "restore is derived from one validated quarantine record and remains dry-run until exact confirmation"
                .to_string(),
            "occupied destinations, symlinks, source drift, and record drift fail closed"
                .to_string(),
        ],
    };
    let validation = validate_action_plan(&plan);
    if validation.valid {
        Ok(plan)
    } else {
        Err(format!(
            "generated restore action plan is invalid: {:?}",
            validation.errors
        ))
    }
}

fn render_review(review: &RestoreReview, format: OutputFormat) -> (ExitCode, String, String) {
    match format {
        OutputFormat::Text => {
            let mut output = format!("{} restore review\n\n", brand::TITLE);
            let _ = writeln!(output, "mode: dry-run read-only");
            let _ = writeln!(output, "contract: {}", review.contract);
            let _ = writeln!(output, "plan_id: {}", review.plan_id);
            let _ = writeln!(
                output,
                "action_id: {}",
                review.action_plan.actions[0].action_id
            );
            let _ = writeln!(output, "writes_attempted: no");
            let _ = writeln!(output, "execution_authorized: no");
            output.push_str("warnings:\n");
            for warning in &review.warnings {
                let _ = writeln!(output, "  - {warning}");
            }
            (ExitCode::Ok, output, String::new())
        }
        OutputFormat::Json => match serde_json::to_string_pretty(review) {
            Ok(json) => (ExitCode::Ok, format!("{json}\n"), String::new()),
            Err(error) => (
                ExitCode::Usage,
                String::new(),
                format!("restore review JSON rendering failed: {error}\n"),
            ),
        },
    }
}

fn render_effect(
    effect: &rz0_quarantine::FilesystemEffectReport,
    format: OutputFormat,
) -> (ExitCode, String, String) {
    match format {
        OutputFormat::Text => (
            ExitCode::Ok,
            format!(
                "runtime.zero restore execution\n\ntransaction_id: {}\nplan_id: {}\naction_id: {}\nstatus: {:?}\nsource_sha256: {}\nsource_size_bytes: {}\nsource_removed: {}\ndestination_verified: {}\nreceipt_reference: {}\nwrites_attempted: {}\nproduct_execution_authorized: {}\n",
                effect.transaction_id,
                effect.plan_id,
                effect.action_id,
                effect.status,
                effect.source_sha256,
                effect.source_size_bytes,
                effect.source_removed,
                effect.destination_verified,
                effect.receipt_reference,
                effect.writes_attempted,
                effect.product_execution_authorized,
            ),
            String::new(),
        ),
        OutputFormat::Json => match serde_json::to_string_pretty(effect) {
            Ok(json) => (ExitCode::Ok, format!("{json}\n"), String::new()),
            Err(error) => (
                ExitCode::Usage,
                String::new(),
                format!("restore execution JSON rendering failed: {error}\n"),
            ),
        },
    }
}

fn usage() -> String {
    "Usage: rz0 restore --dry-run --plan-id <exact-quarantine-plan-id> [--format text|json]\n       rz0 restore --apply --plan-id <exact-quarantine-plan-id> [--challenge-issued-unix-seconds <seconds>] [--confirm <exact-phrase>] [--format text|json]\n\nRestores one existing runtime.zero quarantine record after digest, record, private-root, destination, and exact-confirmation checks. It never overwrites, recurses, deletes, elevates, or uses network access.\n".to_string()
}

fn short_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{:x}", digest)[..16].to_string()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    #[test]
    fn invalid_restore_record_plan_is_rejected_without_path_disclosure() {
        let store = module_store_plan(None, None, "restore test");
        let error =
            exact_restore_plan(&store, "rz0plan-cache-missing").expect_err("missing record");
        assert!(!error.contains("/Users/"));
    }

    #[cfg(unix)]
    #[test]
    fn exact_cache_restore_moves_only_after_record_and_confirmation_checks() {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "runtime-zero-restore-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let cache_root = root.join("external-cache");
        let payload = b"restored cache\n";
        let digest = format!("{:x}", Sha256::digest(payload));
        let plan_id = "rz0plan-cache-restore-fixture";
        let quarantine_dir = root.join("quarantine").join(plan_id);
        let cache_file = cache_root.join("first-party-cache/restored-entry");
        for directory in [
            root.clone(),
            cache_root.clone(),
            cache_root.join("first-party-cache"),
            root.join("state"),
            root.join("state/transactions"),
            root.join("state/receipts"),
            root.join("quarantine"),
            quarantine_dir.clone(),
        ] {
            fs::create_dir_all(&directory).expect("restore directory");
            fs::set_permissions(directory, fs::Permissions::from_mode(0o700))
                .expect("private restore directory");
        }
        fs::write(quarantine_dir.join("payload.bin"), payload).expect("payload");
        fs::set_permissions(
            quarantine_dir.join("payload.bin"),
            fs::Permissions::from_mode(0o600),
        )
        .expect("private payload");
        let mut record = QuarantineRecord {
            schema_version: 1,
            contract: "quarantine_record".to_string(),
            transaction_id: "tx.quarantine.restore-fixture.1000".to_string(),
            plan_id: plan_id.to_string(),
            action_id: "quarantine-cache-fixture".to_string(),
            original_path: "workspace/cache/first-party-cache/restored-entry".to_string(),
            quarantine_path: format!("quarantine/{plan_id}/payload.bin"),
            sha256: digest,
            size_bytes: payload.len() as u64,
            created_unix_seconds: 900,
            binding_sha256: String::new(),
        };
        rz0_quarantine::seal_quarantine_record(&mut record);
        let record_path = quarantine_dir.join("quarantine.json");
        fs::write(
            &record_path,
            serde_json::to_vec(&record).expect("record bytes"),
        )
        .expect("record");
        fs::set_permissions(&record_path, fs::Permissions::from_mode(0o600))
            .expect("private record");
        let mut store = crate::module_store::module_store_plan_for_data_root(
            root.clone(),
            None,
            None,
            "restore test",
        );
        store.cache_root = cache_root.display().to_string();
        let action_plan = exact_restore_plan(&store, plan_id).expect("restore plan");
        assert!(validate_action_plan(&action_plan).valid);
        let challenge = build_exact_quarantine_challenge(&action_plan, 1_000).expect("challenge");
        let response =
            validate_exact_quarantine_confirmation(&challenge, &challenge.expected_phrase, 1_100)
                .expect("confirmation");
        let effect = execute_exact_quarantine(&store, &action_plan, &challenge, &response, 1_100)
            .expect("restore effect");
        assert!(!effect.source_removed);
        assert_eq!(fs::read(&cache_file).expect("restored file"), payload);
        assert!(!quarantine_dir.join("payload.bin").exists());
        let _ = fs::remove_dir_all(root);
    }
}
