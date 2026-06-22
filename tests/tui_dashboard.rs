use runtime_zero::install_receipt::ReceiptInventoryState;
use runtime_zero::installed_registry::InstalledRegistryState;
use runtime_zero::store_init::StoreInitStatus;
use runtime_zero::tui_dashboard;

#[test]
fn dashboard_does_not_claim_active_feature_modules() {
    let dashboard = tui_dashboard::dashboard();
    assert_eq!(dashboard.installed_module_count, 0);
    assert!(matches!(
        dashboard.registry_state,
        InstalledRegistryState::Absent | InstalledRegistryState::Valid
    ));
    assert!(matches!(
        dashboard.receipt_state,
        ReceiptInventoryState::NotReferenced | ReceiptInventoryState::Valid
    ));
    assert!(matches!(
        dashboard.store_init_status,
        StoreInitStatus::Ready | StoreInitStatus::AlreadyInitialized
    ));
    assert!(dashboard.planned_module_family_count > 0);
}
