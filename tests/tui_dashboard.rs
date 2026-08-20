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
    assert_eq!(dashboard.update_check_status, "not checked");
    assert_eq!(dashboard.update_candidate_count, 0);
    assert!(dashboard.cache_status.starts_with("live") || dashboard.cache_status == "unavailable");
    assert!(
        dashboard.leftovers_status.starts_with("live")
            || dashboard.leftovers_status == "unavailable"
    );
    let diagnostics = dashboard
        .sections
        .iter()
        .find(|section| section.title == "diagnostics")
        .expect("diagnostics section");
    assert!(
        diagnostics
            .rows
            .iter()
            .any(|row| row.value.contains("bounded evidence")
                || row.value.contains("cache/leftovers evidence unavailable"))
    );
    let monitor = dashboard
        .sections
        .iter()
        .find(|section| section.title == "system")
        .expect("system monitor section");
    assert!(monitor.summary.contains("CPU"));
    assert!(monitor.rows.iter().any(|row| row.value.contains("memory")));

    #[cfg(target_os = "macos")]
    {
        assert!(dashboard.installed_software_count > 0);
        let installed = dashboard
            .sections
            .iter()
            .find(|section| section.title == "software")
            .expect("software section");
        assert!(installed.rows.iter().any(|row| row.label == "[APP]"));
        assert!(installed.rows.iter().any(|row| {
            row.preview
                .as_deref()
                .is_some_and(|preview| preview.contains("rz0 uninstall plan"))
        }));
        assert!(installed.rows.iter().any(|row| {
            row.value.contains("system protected")
                && row
                    .preview
                    .as_deref()
                    .is_some_and(|preview| !preview.contains("rz0 uninstall plan"))
        }));
        assert!(
            dashboard
                .sections
                .iter()
                .all(|section| section.title != "uninstall options")
        );
    }
}
