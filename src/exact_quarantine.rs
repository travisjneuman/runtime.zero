use std::collections::BTreeSet;
use std::time::{SystemTime, UNIX_EPOCH};

use rz0_action_plan::{
    ActionCapability, ActionDisposition, ActionKind, ActionPlan, ActionRisk, ActionSource,
    PlanAction, RollbackPlan, WriteKind, WriteSetEntry, action_plan_digests, validate_action_plan,
};
use rz0_confirmation_contract::{
    ConfirmationChallenge, ConfirmationConsumption, ConfirmationResponse, ConfirmationRisk,
    ConfirmationSurface, confirmation_response_sha256, seal_confirmation_challenge,
    seal_confirmation_consumption, validate_confirmation,
};
use rz0_finding_contract::{FindingDisposition, FindingReport};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    module_store::ModuleStorePlan,
    quarantine::{
        FilesystemEffectError, FilesystemEffectReport, FilesystemEffectRequest,
        execute_filesystem_effect, filesystem_effect_transaction_id,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ExactQuarantineChallengeView {
    schema_version: u16,
    contract: &'static str,
    plan_id: String,
    action_id: String,
    plan_sha256: String,
    issued_unix_seconds: u64,
    expires_unix_seconds: u64,
    expected_phrase: String,
    writes_attempted: bool,
    execution_authorized: bool,
}

pub(crate) struct ExactQuarantineActionSpec<'a> {
    pub module_id: &'a str,
    pub target: &'a str,
    pub source_path: String,
    pub source_sha256: String,
    pub source_size_bytes: u64,
    pub finding_report: &'a FindingReport,
}

pub(crate) fn build_exact_quarantine_action_plan(
    spec: ExactQuarantineActionSpec<'_>,
) -> Result<ActionPlan, String> {
    let finding = spec
        .finding_report
        .findings
        .iter()
        .find(|finding| {
            finding.disposition == FindingDisposition::QuarantineCandidate
                && finding.exact_evidence.as_ref().is_some_and(|evidence| {
                    evidence.sha256 == spec.source_sha256
                        && evidence.size_bytes == spec.source_size_bytes
                })
        })
        .ok_or_else(|| {
            "exact quarantine plan requires one matching candidate finding".to_string()
        })?;
    if !spec.source_path.starts_with("workspace/") {
        return Err("exact quarantine source must use the workspace namespace".to_string());
    }
    let suffix = spec
        .module_id
        .rsplit('.')
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or("module");
    let short = short_digest(spec.source_sha256.as_bytes());
    let plan_id = format!("rz0plan-{suffix}-{short}");
    let action_id = format!("quarantine-{suffix}-{short}");
    let quarantine_prefix = format!("quarantine/{plan_id}");
    let action_plan = ActionPlan {
        schema_version: rz0_action_plan::ACTION_PLAN_SCHEMA_VERSION,
        plan_id,
        module_id: spec.module_id.to_string(),
        created_at: None,
        expires_at: None,
        dry_run: true,
        writes_attempted: false,
        evidence_contract: rz0_finding_contract::FINDING_CONTRACT.to_string(),
        evidence_report_id: spec.finding_report.report_id.clone(),
        evidence_sha256: spec.finding_report.input_evidence_sha256.clone(),
        actions: vec![PlanAction {
            action_id,
            finding_id: finding.finding_id.clone(),
            kind: ActionKind::Quarantine,
            disposition: ActionDisposition::Planned,
            target: spec.target.to_string(),
            source: Some(ActionSource {
                path: spec.source_path,
                sha256: spec.source_sha256,
                size_bytes: spec.source_size_bytes,
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
            capabilities: vec![ActionCapability::RuntimeStateWrite, ActionCapability::QuarantineWrite],
            forbidden_path_classes: Vec::new(),
            write_set: vec![
                WriteSetEntry {
                    path: format!("{quarantine_prefix}/payload.bin"),
                    kind: WriteKind::QuarantinedPayload,
                },
                WriteSetEntry {
                    path: format!("{quarantine_prefix}/quarantine.json"),
                    kind: WriteKind::QuarantineRecord,
                },
            ],
            rollback: RollbackPlan {
                supported: true,
                quarantine_required: true,
                description:
                    "Restore only to the exact original logical path after fresh digest verification."
                        .to_string(),
            },
        }],
        warnings: vec![
            "exact path was supplied explicitly; this plan is dry-run only and does not move files"
                .to_string(),
            "the absolute source root is intentionally withheld from the report; execution requires a separate reviewed CLI/TUI binding"
                .to_string(),
        ],
    };
    let validation = validate_action_plan(&action_plan);
    if !validation.valid {
        return Err(format!(
            "generated action plan is invalid: {:?}",
            validation.errors
        ));
    }
    action_plan_digests(&action_plan)
        .map_err(|errors| format!("generated action plan digest failed: {errors:?}"))?;
    Ok(action_plan)
}

pub(crate) fn build_exact_quarantine_challenge(
    plan: &ActionPlan,
    issued_unix_seconds: u64,
) -> Result<ConfirmationChallenge, String> {
    let digests = action_plan_digests(plan).map_err(|errors| errors.join("; "))?;
    let action = plan
        .actions
        .first()
        .ok_or_else(|| "exact quarantine plan contains no action".to_string())?;
    let capabilities = action
        .capabilities
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut challenge = ConfirmationChallenge {
        schema_version: rz0_confirmation_contract::CONFIRMATION_SCHEMA_VERSION,
        contract: rz0_confirmation_contract::CONFIRMATION_CHALLENGE_CONTRACT.to_string(),
        challenge_id: format!(
            "challenge.quarantine.{}",
            short_digest(plan.plan_id.as_bytes())
        ),
        plan_id: plan.plan_id.clone(),
        plan_sha256: digests.plan_sha256.clone(),
        dry_run_sha256: digests.plan_sha256,
        write_set_sha256: digests.write_set_sha256,
        before_state_sha256: Some(digest_text(&format!("{}\0before", action.action_id))),
        expected_after_state_sha256: digest_text(&format!("{}\0after", action.action_id)),
        risk: ConfirmationRisk::Mutating,
        action_count: 1,
        capabilities,
        issued_unix_seconds,
        expires_unix_seconds: issued_unix_seconds.saturating_add(300),
        dry_run_completed: true,
        dry_run_writes_attempted: false,
        rollback_available: action.rollback.supported,
        quarantine_available: true,
        manual_recovery_acknowledged: false,
        expected_phrase: String::new(),
        challenge_sha256: String::new(),
    };
    seal_confirmation_challenge(&mut challenge);
    Ok(challenge)
}

pub(crate) fn validate_exact_quarantine_confirmation(
    challenge: &ConfirmationChallenge,
    phrase: &str,
    now_unix_seconds: u64,
) -> Result<ConfirmationResponse, String> {
    let response = ConfirmationResponse {
        schema_version: rz0_confirmation_contract::CONFIRMATION_SCHEMA_VERSION,
        contract: rz0_confirmation_contract::CONFIRMATION_RESPONSE_CONTRACT.to_string(),
        challenge_id: challenge.challenge_id.clone(),
        challenge_sha256: challenge.challenge_sha256.clone(),
        confirmed_unix_seconds: now_unix_seconds,
        surface: ConfirmationSurface::Cli,
        phrase: phrase.to_string(),
        interactive: true,
        single_use: true,
        execution_authorized: false,
    };
    let assessment = validate_confirmation(challenge, &response, now_unix_seconds);
    if assessment.valid {
        Ok(response)
    } else {
        Err(assessment.errors.join("; "))
    }
}

pub(crate) fn build_exact_quarantine_consumption(
    plan: &ActionPlan,
    challenge: &ConfirmationChallenge,
    response: &ConfirmationResponse,
    consumed_unix_seconds: u64,
) -> ConfirmationConsumption {
    let mut consumption = ConfirmationConsumption {
        schema_version: rz0_confirmation_contract::CONFIRMATION_SCHEMA_VERSION,
        contract: rz0_confirmation_contract::CONFIRMATION_CONSUMPTION_CONTRACT.to_string(),
        transaction_id: filesystem_effect_transaction_id(
            plan.actions
                .first()
                .map_or(ActionKind::Quarantine, |action| action.kind),
            &plan.plan_id,
            challenge.issued_unix_seconds,
        ),
        plan_id: plan.plan_id.clone(),
        challenge_sha256: challenge.challenge_sha256.clone(),
        response_sha256: confirmation_response_sha256(response),
        consumed_unix_seconds,
        single_use_consumed: true,
        execution_authorized: false,
        binding_sha256: String::new(),
    };
    seal_confirmation_consumption(&mut consumption);
    consumption
}

pub(crate) fn execute_exact_quarantine(
    store: &ModuleStorePlan,
    plan: &ActionPlan,
    challenge: &ConfirmationChallenge,
    response: &ConfirmationResponse,
    consumed_unix_seconds: u64,
) -> Result<FilesystemEffectReport, FilesystemEffectError> {
    let consumption =
        build_exact_quarantine_consumption(plan, challenge, response, consumed_unix_seconds);
    let workspace_namespace =
        workspace_namespace(plan).map_err(rz0_quarantine::FilesystemEffectError::invalid_plan)?;
    let workspace_root = if workspace_namespace == "workspace/cache" {
        std::path::Path::new(&store.cache_root)
    } else {
        std::path::Path::new(&store.data_root)
    };
    execute_filesystem_effect(FilesystemEffectRequest {
        state_root: std::path::Path::new(&store.state_root),
        source_root: workspace_root,
        quarantine_root: std::path::Path::new(&store.quarantine_root),
        plan,
        action: &plan.actions[0],
        challenge,
        response,
        consumption: &consumption,
        workspace_namespace: Some(workspace_namespace),
        cancellation: None,
        now_unix_seconds: challenge.issued_unix_seconds,
    })
}

fn workspace_namespace(plan: &ActionPlan) -> Result<&'static str, String> {
    let action = plan
        .actions
        .first()
        .ok_or_else(|| "exact filesystem plan contains no action".to_string())?;
    let path = match action.kind {
        ActionKind::Quarantine => action
            .source
            .as_ref()
            .map(|source| source.path.as_str())
            .ok_or_else(|| "exact quarantine action contains no source".to_string())?,
        ActionKind::Restore => action
            .write_set
            .iter()
            .find(|entry| entry.kind == WriteKind::RestoredPayload)
            .map(|entry| entry.path.as_str())
            .ok_or_else(|| "exact restore action contains no destination".to_string())?,
        _ => {
            return Err("exact filesystem action must be quarantine or restore".to_string());
        }
    };
    if path.starts_with("workspace/cache/") {
        Ok("workspace/cache")
    } else if path.starts_with("workspace/") {
        Ok("workspace")
    } else {
        Err("exact filesystem action must remain under workspace".to_string())
    }
}

pub(crate) fn render_exact_quarantine_challenge(
    challenge: &ConfirmationChallenge,
    action_id: &str,
    json: bool,
) -> String {
    render_exact_filesystem_challenge(challenge, action_id, "quarantine", json)
}

pub(crate) fn render_exact_filesystem_challenge(
    challenge: &ConfirmationChallenge,
    action_id: &str,
    operation: &str,
    json: bool,
) -> String {
    let view = ExactQuarantineChallengeView {
        schema_version: rz0_confirmation_contract::CONFIRMATION_SCHEMA_VERSION,
        contract: rz0_confirmation_contract::CONFIRMATION_CHALLENGE_CONTRACT,
        plan_id: challenge.plan_id.clone(),
        action_id: action_id.to_string(),
        plan_sha256: challenge.plan_sha256.clone(),
        issued_unix_seconds: challenge.issued_unix_seconds,
        expires_unix_seconds: challenge.expires_unix_seconds,
        expected_phrase: challenge.expected_phrase.clone(),
        writes_attempted: false,
        execution_authorized: false,
    };
    if json {
        serde_json::to_string_pretty(&view).map_or_else(
            |error| format!("challenge serialization failed: {error}\n"),
            |json| format!("{json}\n"),
        )
    } else {
        format!(
            "runtime.zero {operation} confirmation\n\nplan_id: {}\naction_id: {}\nplan_sha256: {}\nissued_unix_seconds: {}\nexpires_unix_seconds: {}\n\nType this exact phrase in a new command invocation with --challenge-issued-unix-seconds {} and --confirm:\n{}\n\nNo file was moved.\n",
            view.plan_id,
            view.action_id,
            view.plan_sha256,
            view.issued_unix_seconds,
            view.expires_unix_seconds,
            view.issued_unix_seconds,
            view.expected_phrase,
        )
    }
}

pub(crate) fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn digest_text(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn short_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{:x}", digest)[..16].to_string()
}
