use rz0_action_plan::{ActionPlan, validate_action_plan};

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
fn unknown_fields_fail_deserialization() {
    let source = include_str!("fixtures/valid-update.json").replacen(
        "\"schema_version\": 1,",
        "\"schema_version\": 1, \"surprise\": true,",
        1,
    );
    assert!(serde_json::from_str::<ActionPlan>(&source).is_err());
}
