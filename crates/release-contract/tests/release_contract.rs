use rz0_release_contract::{
    AcceptanceStatus, Architecture, ArtifactKind, PlatformFamily, RELEASE_CONTRACT,
    RELEASE_SCHEMA_VERSION, ReleaseAcceptanceAssessment, ReleaseDecision, ReleaseTarget,
    SupportTier, missing_cells_for_targets, summarize_acceptance, validate_release_assessment,
};

#[test]
fn exact_cross_product_is_valid_bounded_and_blocked() {
    let assessment = assessment(targets());
    let validation = validate_release_assessment(&assessment);
    assert!(validation.valid, "{:?}", validation.errors);
    let summary = summarize_acceptance(&assessment);
    assert_eq!(summary.targets, 2);
    assert_eq!(summary.cells, 168);
    assert_eq!(summary.missing, 168);
    assert_eq!(summary.proven, 0);
    assert!(!assessment.release_authorized);
}

#[test]
fn target_order_duplication_and_retired_release_tier_fail_closed() {
    let mut entries = targets();
    entries.reverse();
    entries[0].tier = SupportTier::ReleaseBlocking;
    entries[0].vendor_supported = false;
    let mut assessment = assessment(entries);
    assessment.targets[1].target_id = assessment.targets[0].target_id.clone();
    let validation = validate_release_assessment(&assessment);
    assert!(!validation.valid);
    for expected in ["unique", "sorted", "vendor-retired"] {
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
fn missing_reordered_or_mismatched_cells_fail_closed() {
    let mut assessment = assessment(targets());
    assessment.cells.swap(0, 1);
    assessment.cells.pop();
    let validation = validate_release_assessment(&assessment);
    assert!(!validation.valid);
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("exact"))
    );
}

#[test]
fn evidence_shapes_are_strict_and_schema_one_never_authorizes_release() {
    let mut assessment = assessment(targets());
    assessment.cells[0].status = AcceptanceStatus::Proven;
    assessment.cells[0].mechanism = Some("native artifact-only runtime test".to_string());
    assessment.cells[0].evidence_reference = Some("evidence.windows11.home.x64.001".to_string());
    assert!(validate_release_assessment(&assessment).valid);

    assessment.cells[1].status = AcceptanceStatus::NotApplicable;
    assessment.cells[1].rationale = Some("module stage does not mutate state".to_string());
    assessment.cells[1].evidence_reference = Some("review.na.inventory.001".to_string());
    assert!(validate_release_assessment(&assessment).valid);

    for cell in &mut assessment.cells {
        cell.status = AcceptanceStatus::Proven;
        cell.mechanism = Some("synthetic proof cannot authorize release".to_string());
        cell.evidence_reference = Some("evidence.synthetic.001".to_string());
        cell.rationale = None;
    }
    assert!(validate_release_assessment(&assessment).valid);
    assert!(!assessment.release_authorized);
}

#[test]
fn unknown_fields_and_fabricated_authorization_fail() {
    let assessment = assessment(targets());
    let json = serde_json::to_string(&assessment).expect("serialize assessment");
    let unknown = json.replacen(
        "\"schema_version\":1",
        "\"schema_version\":1,\"surprise\":true",
        1,
    );
    assert!(serde_json::from_str::<ReleaseAcceptanceAssessment>(&unknown).is_err());
    let fabricated = json.replace(
        "\"release_authorized\":false",
        "\"release_authorized\":true",
    );
    let value: ReleaseAcceptanceAssessment =
        serde_json::from_str(&fabricated).expect("authorization has valid shape");
    assert!(!validate_release_assessment(&value).valid);
}

fn assessment(targets: Vec<ReleaseTarget>) -> ReleaseAcceptanceAssessment {
    let cells = missing_cells_for_targets(&targets);
    ReleaseAcceptanceAssessment {
        schema_version: RELEASE_SCHEMA_VERSION,
        contract: RELEASE_CONTRACT.to_string(),
        assessment_id: "rz0release-first-round".to_string(),
        scope_revision: "scope-2026-07-29".to_string(),
        decision: ReleaseDecision::Blocked,
        release_authorized: false,
        targets,
        cells,
    }
}

fn targets() -> Vec<ReleaseTarget> {
    vec![
        ReleaseTarget {
            target_id: "macos-26-default-arm64-zip".to_string(),
            platform: PlatformFamily::Macos,
            generation: "macOS Tahoe 26".to_string(),
            variant: "default".to_string(),
            architecture: Architecture::Arm64,
            artifact: ArtifactKind::PortableZip,
            tier: SupportTier::ReleaseBlocking,
            vendor_supported: true,
        },
        ReleaseTarget {
            target_id: "windows-7-home-premium-x86_64-zip".to_string(),
            platform: PlatformFamily::WindowsClient,
            generation: "Windows 7 SP1".to_string(),
            variant: "Home Premium".to_string(),
            architecture: Architecture::X86_64,
            artifact: ArtifactKind::PortableZip,
            tier: SupportTier::LegacyCompatibility,
            vendor_supported: false,
        },
    ]
}
