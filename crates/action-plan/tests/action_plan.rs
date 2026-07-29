use rz0_action_plan::{ActionCapability, ActionPlan, action_plan_digests, validate_action_plan};

#[test]
fn valid_update_fixture_is_dry_run_only() {
    let plan: ActionPlan =
        serde_json::from_str(include_str!("fixtures/valid-update.json")).expect("valid fixture");
    let validation = validate_action_plan(&plan);
    assert!(validation.valid, "{:?}", validation.errors);
    assert!(plan.dry_run);
    assert!(!plan.writes_attempted);
    assert!(plan.actions.iter().all(|action| !action.would_write));
}

#[test]
fn uninstall_and_quarantine_fixtures_remain_plans_only() {
    for source in [
        include_str!("fixtures/valid-uninstall.json"),
        include_str!("fixtures/valid-quarantine.json"),
        include_str!("fixtures/valid-restore.json"),
    ] {
        let plan: ActionPlan = serde_json::from_str(source).expect("valid plan fixture");
        let validation = validate_action_plan(&plan);
        assert!(validation.valid, "{:?}", validation.errors);
        assert!(plan.dry_run);
        assert!(!plan.writes_attempted);
        assert!(plan.actions.iter().all(|action| !action.would_write));
    }
}

#[test]
fn quarantine_fixture_binds_exact_source_evidence() {
    let plan: ActionPlan =
        serde_json::from_str(include_str!("fixtures/valid-quarantine.json")).expect("fixture");
    let validation = validate_action_plan(&plan);
    assert!(validation.valid, "{:?}", validation.errors);
    let source = plan.actions[0].source.as_ref().expect("source evidence");
    assert_eq!(source.path, "workspace/stale-shim.bin");
    assert_eq!(source.size_bytes, 30);
}

#[test]
fn quarantine_source_evidence_fails_closed_on_drift() {
    let mut plan: ActionPlan =
        serde_json::from_str(include_str!("fixtures/valid-quarantine.json")).expect("fixture");
    let source = plan.actions[0].source.as_mut().expect("source");
    source.path = "../outside".to_string();
    source.sha256 = "A".repeat(64);
    source.size_bytes = 64 * 1024 * 1024 + 1;
    let validation = validate_action_plan(&plan);
    assert!(!validation.valid);
    for expected in ["source path", "source sha256", "source exceeds"] {
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains(expected)),
            "missing {expected}: {:?}",
            validation.errors
        );
    }
}

#[test]
fn invalid_write_fixture_fails_multiple_policy_gates() {
    let plan: ActionPlan =
        serde_json::from_str(include_str!("fixtures/invalid-write.json")).expect("shape");
    let validation = validate_action_plan(&plan);
    assert!(!validation.valid);
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("would_write"))
    );
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("confirmation"))
    );
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("absolute executable"))
    );
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("forbidden"))
    );
}

#[test]
fn blocked_sensitive_finding_is_valid_report_only_evidence() {
    let plan: ActionPlan = serde_json::from_str(include_str!("fixtures/blocked-credential.json"))
        .expect("blocked fixture");
    let validation = validate_action_plan(&plan);
    assert!(validation.valid, "{:?}", validation.errors);
    assert_eq!(
        plan.actions[0].disposition,
        rz0_action_plan::ActionDisposition::Blocked
    );
    assert!(plan.actions[0].write_set.is_empty());
}

#[test]
fn read_only_protocol_capabilities_are_rejected_by_action_schema() {
    let mut plan: ActionPlan =
        serde_json::from_str(include_str!("fixtures/valid-update.json")).expect("fixture");
    plan.actions[0]
        .capabilities
        .insert(0, ActionCapability::ProcessEnvironmentRead);
    let validation = validate_action_plan(&plan);
    assert!(!validation.valid);
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("outside action-plan schema"))
    );
}

#[test]
fn plan_and_write_set_digests_are_deterministic_and_domain_separated() {
    let plan: ActionPlan =
        serde_json::from_str(include_str!("fixtures/valid-update.json")).expect("fixture");
    let first = action_plan_digests(&plan).expect("digests");
    let second = action_plan_digests(&plan).expect("digests");
    assert_eq!(first, second);
    assert_ne!(first.plan_sha256, first.write_set_sha256);
    assert_eq!(first.plan_sha256.len(), 64);
}

#[test]
fn plan_metadata_and_write_set_drift_have_distinct_digest_effects() {
    let plan: ActionPlan =
        serde_json::from_str(include_str!("fixtures/valid-update.json")).expect("fixture");
    let baseline = action_plan_digests(&plan).expect("baseline");

    let mut warning_drift = plan.clone();
    warning_drift
        .warnings
        .push("operator-visible warning".to_string());
    let warning = action_plan_digests(&warning_drift).expect("warning digest");
    assert_ne!(warning.plan_sha256, baseline.plan_sha256);
    assert_eq!(warning.write_set_sha256, baseline.write_set_sha256);

    let write_plan: ActionPlan =
        serde_json::from_str(include_str!("fixtures/valid-quarantine.json")).expect("fixture");
    let write_baseline = action_plan_digests(&write_plan).expect("write baseline");
    let mut write_drift = write_plan;
    write_drift.actions[0].write_set[0].path = "quarantine/other-stale-shim.bin".to_string();
    let write = action_plan_digests(&write_drift).expect("write digest");
    assert_ne!(write.plan_sha256, write_baseline.plan_sha256);
    assert_ne!(write.write_set_sha256, write_baseline.write_set_sha256);
}

#[test]
fn invalid_plans_cannot_receive_foundation_digests() {
    let mut plan: ActionPlan =
        serde_json::from_str(include_str!("fixtures/valid-update.json")).expect("fixture");
    plan.writes_attempted = true;
    assert!(action_plan_digests(&plan).is_err());
}

#[test]
fn unknown_fields_fail_deserialization() {
    let source = include_str!("fixtures/valid-update.json").replacen(
        "\"schema_version\": 1,",
        "\"schema_version\": 1, \"surprise\": true,",
        1,
    );
    assert!(serde_json::from_str::<ActionPlan>(&source).is_err());
}
