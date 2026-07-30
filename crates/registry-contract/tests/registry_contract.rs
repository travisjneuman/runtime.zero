use rz0_registry_contract::{
    InstalledModuleRecord, InstalledRegistry, RegistryDocumentErrorCode, RegistryViolationCode,
    canonical_registry_bytes, parse_registry_document, registry_sha256, validate_registry,
};

fn record(id: &str) -> InstalledModuleRecord {
    InstalledModuleRecord {
        id: id.to_string(),
        version: "0.1.0".to_string(),
        manifest_path: format!("modules/{id}/0.1.0/rz0-module.json"),
        receipt_path: format!("receipts/{id}.json"),
        module_dir: Some(format!("modules/{id}/0.1.0")),
    }
}

#[test]
fn canonical_registry_round_trips_with_stable_bytes_and_digest() {
    let registry = InstalledRegistry {
        schema_version: 1,
        modules: vec![
            record("first-party.inventory"),
            record("first-party.leftovers"),
        ],
    };
    let bytes = canonical_registry_bytes(&registry).unwrap();
    assert_eq!(parse_registry_document(&bytes).unwrap(), registry);
    assert_eq!(registry_sha256(&registry).unwrap().len(), 64);
    assert_eq!(bytes, canonical_registry_bytes(&registry).unwrap());
}

#[test]
fn exact_module_paths_order_and_uniqueness_are_required() {
    let mut registry = InstalledRegistry {
        schema_version: 1,
        modules: vec![
            record("first-party.leftovers"),
            record("first-party.inventory"),
        ],
    };
    registry.modules[1].manifest_path =
        "modules/first-party.inventory/other/rz0-module.json".to_string();
    let validation = validate_registry(&registry);
    assert!(!validation.valid);
    assert!(
        validation
            .violations
            .iter()
            .any(|error| error.code == RegistryViolationCode::NonCanonicalOrder)
    );
    assert!(
        validation
            .violations
            .iter()
            .any(|error| error.code == RegistryViolationCode::InvalidManifestPath)
    );

    registry.modules[1] = registry.modules[0].clone();
    let validation = validate_registry(&registry);
    assert!(
        validation
            .violations
            .iter()
            .any(|error| error.code == RegistryViolationCode::DuplicateModuleId)
    );
}

#[test]
fn malformed_unknown_and_oversized_documents_fail_closed() {
    let malformed = parse_registry_document(br#"{"schema_version":1,"modules":[]"#).unwrap_err();
    assert_eq!(malformed.code, RegistryDocumentErrorCode::Malformed);
    let unknown =
        parse_registry_document(br#"{"schema_version":1,"modules":[],"release_authorized":true}"#)
            .unwrap_err();
    assert_eq!(unknown.code, RegistryDocumentErrorCode::Malformed);
    let oversized = vec![b' '; rz0_resource_contract::MAX_REGISTRY_DOCUMENT_BYTES as usize + 1];
    assert_eq!(
        parse_registry_document(&oversized).unwrap_err().code,
        RegistryDocumentErrorCode::LimitExceeded
    );
}

#[test]
fn reserved_ids_unsafe_receipts_and_version_drift_are_rejected() {
    let mut item = record("core.inventory");
    item.version = "0.1.0 unsafe".to_string();
    item.receipt_path = "../receipt.json".to_string();
    let validation = validate_registry(&InstalledRegistry {
        schema_version: 1,
        modules: vec![item],
    });
    for code in [
        RegistryViolationCode::ReservedModuleId,
        RegistryViolationCode::InvalidVersion,
        RegistryViolationCode::InvalidReceiptPath,
    ] {
        assert!(validation.violations.iter().any(|item| item.code == code));
    }
}
