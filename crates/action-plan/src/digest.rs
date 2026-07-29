use sha2::{Digest, Sha256};

use crate::{ActionPlan, WriteKind, validate_action_plan};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionPlanDigests {
    pub plan_sha256: String,
    pub write_set_sha256: String,
}

pub fn action_plan_digests(plan: &ActionPlan) -> Result<ActionPlanDigests, Vec<String>> {
    let validation = validate_action_plan(plan);
    if !validation.valid {
        return Err(validation.errors);
    }
    let serialized = serde_json::to_vec(plan)
        .map_err(|error| vec![format!("serialize validated action plan: {error}")])?;
    let mut plan_digest = Sha256::new();
    plan_digest.update(b"runtime.zero.action-plan.v1\0");
    plan_digest.update((serialized.len() as u64).to_be_bytes());
    plan_digest.update(&serialized);

    let mut write_set_digest = Sha256::new();
    write_set_digest.update(b"runtime.zero.action-plan-write-set.v1\0");
    write_set_digest.update((plan.actions.len() as u64).to_be_bytes());
    for action in &plan.actions {
        put(&mut write_set_digest, &action.action_id);
        write_set_digest.update((action.write_set.len() as u64).to_be_bytes());
        for entry in &action.write_set {
            put(&mut write_set_digest, &entry.path);
            put(&mut write_set_digest, write_kind_name(entry.kind));
        }
    }

    Ok(ActionPlanDigests {
        plan_sha256: format!("{:x}", plan_digest.finalize()),
        write_set_sha256: format!("{:x}", write_set_digest.finalize()),
    })
}

fn put(digest: &mut Sha256, value: &str) {
    digest.update((value.len() as u64).to_be_bytes());
    digest.update(value.as_bytes());
}

fn write_kind_name(kind: WriteKind) -> &'static str {
    match kind {
        WriteKind::RuntimeState => "runtime_state",
        WriteKind::QuarantineRecord => "quarantine_record",
        WriteKind::QuarantinedPayload => "quarantined_payload",
        WriteKind::RestoredPayload => "restored_payload",
    }
}
