use crate::install_receipt::ReceiptInventoryState;
use crate::installed_registry::InstalledRegistryState;
use crate::store_init::StoreInitStatus;
use crate::store_status::StoreOverallState;
use crate::tui_dashboard::TuiRow;
use crate::tui_theme;

pub(crate) fn row(label: &'static str, value: &str, tone: &'static str) -> TuiRow {
    TuiRow {
        label,
        value: value.to_string(),
        tone,
    }
}

pub(crate) fn row_count(
    label: &'static str,
    count: usize,
    suffix: &str,
    tone: &'static str,
) -> TuiRow {
    row(label, &format!("{count} {suffix}"), tone)
}

pub(crate) fn store_state_label(state: StoreOverallState) -> &'static str {
    match state {
        StoreOverallState::NotInitialized => "store not initialized",
        StoreOverallState::Empty => "store paths exist but are empty",
        StoreOverallState::Present => "store paths present",
        StoreOverallState::Invalid => "store path mismatch detected",
    }
}

pub(crate) fn init_status_label(status: StoreInitStatus) -> &'static str {
    match status {
        StoreInitStatus::Ready => "store init dry-run ready",
        StoreInitStatus::AlreadyInitialized => "store scaffolding initialized",
        StoreInitStatus::Applied => "store init applied",
        StoreInitStatus::Blocked => "store init blocked",
    }
}

pub(crate) fn init_label(status: StoreInitStatus) -> &'static str {
    match status {
        StoreInitStatus::Blocked => tui_theme::LABEL_WARN,
        StoreInitStatus::AlreadyInitialized | StoreInitStatus::Applied => tui_theme::LABEL_OK,
        StoreInitStatus::Ready => tui_theme::LABEL_DRY_RUN,
    }
}

pub(crate) fn init_tone(status: StoreInitStatus) -> &'static str {
    match status {
        StoreInitStatus::Blocked => "warn",
        StoreInitStatus::AlreadyInitialized | StoreInitStatus::Applied => "safe",
        StoreInitStatus::Ready => "dry_run",
    }
}

pub(crate) fn registry_state_label(state: InstalledRegistryState) -> &'static str {
    match state {
        InstalledRegistryState::Absent => "registry absent",
        InstalledRegistryState::Empty => "registry file empty",
        InstalledRegistryState::Valid => "registry valid",
        InstalledRegistryState::Invalid => "registry invalid",
        InstalledRegistryState::Unreadable => "registry unreadable",
    }
}

pub(crate) fn registry_label(state: InstalledRegistryState) -> &'static str {
    match state {
        InstalledRegistryState::Invalid | InstalledRegistryState::Unreadable => {
            tui_theme::LABEL_WARN
        }
        InstalledRegistryState::Valid => tui_theme::LABEL_OK,
        InstalledRegistryState::Absent | InstalledRegistryState::Empty => tui_theme::LABEL_INFO,
    }
}

pub(crate) fn registry_tone(state: InstalledRegistryState) -> &'static str {
    match state {
        InstalledRegistryState::Invalid | InstalledRegistryState::Unreadable => "warn",
        InstalledRegistryState::Valid => "safe",
        InstalledRegistryState::Absent | InstalledRegistryState::Empty => "info",
    }
}

pub(crate) fn receipt_state_label(state: ReceiptInventoryState) -> &'static str {
    match state {
        ReceiptInventoryState::NotReferenced => "receipts not referenced",
        ReceiptInventoryState::Absent => "receipt missing",
        ReceiptInventoryState::Valid => "receipts valid",
        ReceiptInventoryState::Invalid => "receipt invalid",
        ReceiptInventoryState::Unreadable => "receipt unreadable",
        ReceiptInventoryState::UnsupportedSchema => "receipt unsupported schema",
    }
}

pub(crate) fn receipt_label(state: ReceiptInventoryState) -> &'static str {
    match state {
        ReceiptInventoryState::Invalid
        | ReceiptInventoryState::Unreadable
        | ReceiptInventoryState::UnsupportedSchema
        | ReceiptInventoryState::Absent => tui_theme::LABEL_WARN,
        ReceiptInventoryState::Valid => tui_theme::LABEL_OK,
        ReceiptInventoryState::NotReferenced => tui_theme::LABEL_INFO,
    }
}

pub(crate) fn receipt_tone(state: ReceiptInventoryState) -> &'static str {
    match state {
        ReceiptInventoryState::Invalid
        | ReceiptInventoryState::Unreadable
        | ReceiptInventoryState::UnsupportedSchema
        | ReceiptInventoryState::Absent => "warn",
        ReceiptInventoryState::Valid => "safe",
        ReceiptInventoryState::NotReferenced => "info",
    }
}
