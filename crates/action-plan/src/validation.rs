use std::collections::BTreeSet;

use crate::model::{
    ACTION_PLAN_SCHEMA_VERSION, ActionCapability, ActionDisposition, ActionKind, ActionPlan,
    ActionPlanValidation, ActionRisk, MAX_ACTION_SOURCE_BYTES, MAX_ACTIONS, MAX_ARGUMENTS,
    MAX_WRITE_SET, PlanAction, WriteKind,
};
use crate::path_policy::{valid_sha256, validate_simulation_relative_path};

pub fn validate_action_plan(plan: &ActionPlan) -> ActionPlanValidation {
    let mut validation = ActionPlanValidation {
        valid: true,
        errors: Vec::new(),
        warnings: plan.warnings.clone(),
    };
    if plan.schema_version != ACTION_PLAN_SCHEMA_VERSION {
        validation.fail(format!(
            "schema_version must be {ACTION_PLAN_SCHEMA_VERSION}"
        ));
    }
    validate_id(&plan.plan_id, "plan_id", &mut validation);
    validate_id(&plan.module_id, "module_id", &mut validation);
    if !plan.module_id.starts_with("first-party.") {
        validation.fail("module_id must identify a first-party module");
    }
    if !plan.dry_run || plan.writes_attempted {
        validation.fail("action plan fixtures must be dry-run with writes_attempted false");
    }
    if !valid_sha256(&plan.evidence_sha256) {
        validation.fail("evidence_sha256 must be 64 lowercase hexadecimal characters");
    }
    if plan.actions.is_empty() || plan.actions.len() > MAX_ACTIONS {
        validation.fail(format!(
            "actions must contain between 1 and {MAX_ACTIONS} entries"
        ));
    }

    let mut action_ids = BTreeSet::new();
    for action in plan.actions.iter().take(MAX_ACTIONS) {
        if !action_ids.insert(action.action_id.clone()) {
            validation.fail(format!("duplicate action_id '{}'", action.action_id));
        }
        validate_action(action, &mut validation);
    }
    validation.valid = validation.errors.is_empty();
    validation
}

fn validate_action(action: &PlanAction, validation: &mut ActionPlanValidation) {
    validate_id(&action.action_id, "action_id", validation);
    validate_text(&action.target, "target", 240, validation);
    validate_source(action, validation);
    if action.would_write {
        validation.fail(format!(
            "action '{}' must set would_write false",
            action.action_id
        ));
    }
    if action.disposition == ActionDisposition::Planned && !action.requires_confirmation {
        validation.fail(format!(
            "planned action '{}' must require confirmation",
            action.action_id
        ));
    }
    if action.risk == ActionRisk::Blocked && action.disposition != ActionDisposition::Blocked {
        validation.fail(format!(
            "blocked-risk action '{}' must use blocked disposition",
            action.action_id
        ));
    }
    if !action.forbidden_path_classes.is_empty() && action.disposition == ActionDisposition::Planned
    {
        validation.fail(format!(
            "action '{}' references forbidden path classes",
            action.action_id
        ));
    }
    validate_command(action, validation);
    validate_capabilities(action, validation);
    validate_transaction_shape(action, validation);
    validate_write_set(action, validation);
    validate_text(
        &action.rollback.description,
        "rollback.description",
        500,
        validation,
    );
    if matches!(action.kind, ActionKind::Quarantine | ActionKind::Restore)
        && !action.rollback.supported
    {
        validation.fail(format!(
            "action '{}' requires rollback support",
            action.action_id
        ));
    }
}

fn validate_source(action: &PlanAction, validation: &mut ActionPlanValidation) {
    let requires_source = action.disposition == ActionDisposition::Planned
        && matches!(action.kind, ActionKind::Quarantine | ActionKind::Restore);
    let Some(source) = &action.source else {
        if requires_source {
            validation.fail(format!(
                "action '{}' requires exact source evidence",
                action.action_id
            ));
        }
        return;
    };
    if !matches!(action.kind, ActionKind::Quarantine | ActionKind::Restore) {
        validation.fail(format!(
            "action '{}' must not attach filesystem source evidence",
            action.action_id
        ));
    }
    if validate_simulation_relative_path(&source.path).is_err() {
        validation.fail(format!(
            "action '{}' has an unsafe simulation source path",
            action.action_id
        ));
    }
    let expected_prefix = match action.kind {
        ActionKind::Quarantine => "workspace/",
        ActionKind::Restore => "quarantine/",
        _ => "",
    };
    if !source.path.starts_with(expected_prefix) {
        validation.fail(format!(
            "action '{}' source path must use the expected fixture root",
            action.action_id
        ));
    }
    if !valid_sha256(&source.sha256) {
        validation.fail(format!(
            "action '{}' source sha256 must be lowercase hexadecimal",
            action.action_id
        ));
    }
    if source.size_bytes > MAX_ACTION_SOURCE_BYTES {
        validation.fail(format!(
            "action '{}' source exceeds {MAX_ACTION_SOURCE_BYTES} bytes",
            action.action_id
        ));
    }
}

fn validate_command(action: &PlanAction, validation: &mut ActionPlanValidation) {
    if action.arguments.len() > MAX_ARGUMENTS {
        validation.fail(format!(
            "action '{}' exceeds {MAX_ARGUMENTS} arguments",
            action.action_id
        ));
    }
    for argument in &action.arguments {
        validate_text(argument, "argument", 512, validation);
    }
    if matches!(action.kind, ActionKind::Update | ActionKind::Uninstall)
        && action.disposition == ActionDisposition::Planned
    {
        if action.manager.as_deref().is_none_or(str::is_empty) {
            validation.fail(format!("action '{}' requires a manager", action.action_id));
        }
        match action.executable.as_deref() {
            Some(path) if is_absolute_local_path(path) => {}
            _ => validation.fail(format!(
                "action '{}' requires an exact absolute executable path",
                action.action_id
            )),
        }
    }
}

fn validate_capabilities(action: &PlanAction, validation: &mut ActionPlanValidation) {
    let capabilities = action.capabilities.iter().copied().collect::<BTreeSet<_>>();
    if capabilities.len() != action.capabilities.len() {
        validation.fail(format!(
            "action '{}' has duplicate capabilities",
            action.action_id
        ));
    }
    if capabilities
        .iter()
        .any(|capability| !capability.is_schema1_action_capability())
    {
        validation.fail(format!(
            "action '{}' includes a capability outside action-plan schema 1",
            action.action_id
        ));
    }
    if action.network_required && !capabilities.contains(&ActionCapability::NetworkMetadata) {
        validation.fail(format!(
            "action '{}' requires the network_metadata capability",
            action.action_id
        ));
    }
    if !action.network_required && capabilities.contains(&ActionCapability::NetworkMetadata) {
        validation.fail(format!(
            "action '{}' grants network_metadata without a network requirement",
            action.action_id
        ));
    }
    if action.requires_elevation && !capabilities.contains(&ActionCapability::ElevatedManagerAction)
    {
        validation.fail(format!(
            "action '{}' requires the elevated_manager_action capability",
            action.action_id
        ));
    }
    if !action.requires_elevation && capabilities.contains(&ActionCapability::ElevatedManagerAction)
    {
        validation.fail(format!(
            "action '{}' grants elevation without an elevation requirement",
            action.action_id
        ));
    }
    if matches!(action.kind, ActionKind::Update | ActionKind::Uninstall)
        && action.disposition == ActionDisposition::Planned
        && !capabilities.contains(&ActionCapability::ManagerExecution)
    {
        validation.fail(format!(
            "action '{}' requires the manager_execution capability",
            action.action_id
        ));
    }
}

fn validate_transaction_shape(action: &PlanAction, validation: &mut ActionPlanValidation) {
    if action.disposition != ActionDisposition::Planned {
        return;
    }
    let capabilities = action.capabilities.iter().copied().collect::<BTreeSet<_>>();
    let write_kinds = action
        .write_set
        .iter()
        .map(|entry| entry.kind)
        .collect::<BTreeSet<_>>();
    match action.kind {
        ActionKind::Quarantine => {
            if !capabilities.contains(&ActionCapability::RuntimeStateWrite)
                || !capabilities.contains(&ActionCapability::QuarantineWrite)
                || !write_kinds.contains(&WriteKind::QuarantinedPayload)
                || !write_kinds.contains(&WriteKind::QuarantineRecord)
                || !action.rollback.quarantine_required
            {
                validation.fail(format!(
                    "action '{}' lacks quarantine capabilities, write records, or rollback posture",
                    action.action_id
                ));
            }
        }
        ActionKind::Restore => {
            if !capabilities.contains(&ActionCapability::RestoreWrite)
                || !write_kinds.contains(&WriteKind::RestoredPayload)
            {
                validation.fail(format!(
                    "action '{}' lacks restore capability or restored payload write",
                    action.action_id
                ));
            }
        }
        ActionKind::Update | ActionKind::Uninstall => {}
    }
}

fn validate_write_set(action: &PlanAction, validation: &mut ActionPlanValidation) {
    if action.write_set.len() > MAX_WRITE_SET {
        validation.fail(format!(
            "action '{}' exceeds {MAX_WRITE_SET} write-set entries",
            action.action_id
        ));
    }
    let mut paths = BTreeSet::new();
    for entry in action.write_set.iter().take(MAX_WRITE_SET) {
        validate_text(&entry.path, "write_set.path", 1024, validation);
        if validate_simulation_relative_path(&entry.path).is_err()
            || !write_path_matches_kind(&entry.path, entry.kind)
            || !paths.insert(entry.path.clone())
        {
            validation.fail(format!(
                "action '{}' has an unsafe, mismatched, or duplicate write-set path",
                action.action_id
            ));
        }
    }
}

fn write_path_matches_kind(path: &str, kind: WriteKind) -> bool {
    match kind {
        WriteKind::RuntimeState => path.starts_with("state/"),
        WriteKind::QuarantineRecord | WriteKind::QuarantinedPayload => {
            path.starts_with("quarantine/")
        }
        WriteKind::RestoredPayload => path.starts_with("workspace/"),
    }
}

fn validate_id(value: &str, field: &str, validation: &mut ActionPlanValidation) {
    if value.is_empty()
        || value.len() > 100
        || value.starts_with(['.', '-'])
        || value.ends_with(['.', '-'])
        || value.contains("..")
        || !value.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '.' | '-')
        })
    {
        validation.fail(format!(
            "{field} must use lowercase letters, digits, dots, or hyphens"
        ));
    }
}

fn validate_text(value: &str, field: &str, max_len: usize, validation: &mut ActionPlanValidation) {
    if value.trim().is_empty() || value.len() > max_len || value.chars().any(char::is_control) {
        validation.fail(format!(
            "{field} is empty, too long, or contains control characters"
        ));
    }
}

fn is_absolute_local_path(value: &str) -> bool {
    value.starts_with('/')
        || (value.len() >= 3
            && value.as_bytes()[1] == b':'
            && matches!(value.as_bytes()[2], b'\\' | b'/'))
}
