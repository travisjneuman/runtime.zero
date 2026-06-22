use std::fmt::Write as FmtWrite;

use crate::brand;
use crate::installed_registry::InstalledRegistryState;
use crate::store_init_model::{
    StoreInitKind, StoreInitReport, StoreInitRole, StoreInitStatus, StoreInitStepState,
};

pub fn store_init_text(report: &StoreInitReport) -> String {
    let mut out = format!("{} store init\n\n", brand::TITLE);
    let _ = writeln!(
        out,
        "mode: {}",
        if report.dry_run { "dry-run" } else { "apply" }
    );
    let _ = writeln!(out, "writes_attempted: {}", yes_no(report.writes_attempted));
    let _ = writeln!(out, "status: {}", status_label(report.status));
    let _ = writeln!(
        out,
        "registry_status: {}",
        registry_label(report.registry_status)
    );
    let _ = writeln!(out, "init_marker_path: {}", report.init_marker_path);
    let _ = writeln!(out, "steps:");
    for step in &report.steps {
        let _ = writeln!(
            out,
            "  - {}: {} {} ({})",
            role_label(step.role),
            step_label(step.state),
            kind_label(step.expected_kind),
            step.path
        );
        let _ = writeln!(out, "    detail: {}", step.detail);
    }
    let _ = writeln!(out, "safety: {}", report.safety_note);
    out
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn status_label(status: StoreInitStatus) -> &'static str {
    match status {
        StoreInitStatus::Ready => "ready",
        StoreInitStatus::AlreadyInitialized => "already_initialized",
        StoreInitStatus::Applied => "applied",
        StoreInitStatus::Blocked => "blocked",
    }
}

fn registry_label(status: InstalledRegistryState) -> &'static str {
    match status {
        InstalledRegistryState::Absent => "absent",
        InstalledRegistryState::Empty => "empty",
        InstalledRegistryState::Valid => "valid",
        InstalledRegistryState::Invalid => "invalid",
        InstalledRegistryState::Unreadable => "unreadable",
    }
}

fn role_label(role: StoreInitRole) -> &'static str {
    match role {
        StoreInitRole::DataRoot => "data_root",
        StoreInitRole::StateRoot => "state_root",
        StoreInitRole::CacheRoot => "cache_root",
        StoreInitRole::LogRoot => "log_root",
        StoreInitRole::QuarantineRoot => "quarantine_root",
        StoreInitRole::ModulesRoot => "modules_root",
        StoreInitRole::TransactionsDir => "transactions_dir",
        StoreInitRole::ReceiptsDir => "receipts_dir",
        StoreInitRole::RegistryPath => "registry_path",
        StoreInitRole::InitMarkerPath => "init_marker_path",
    }
}

fn kind_label(kind: StoreInitKind) -> &'static str {
    match kind {
        StoreInitKind::Directory => "directory",
        StoreInitKind::File => "file",
    }
}

fn step_label(state: StoreInitStepState) -> &'static str {
    match state {
        StoreInitStepState::WouldCreate => "would_create",
        StoreInitStepState::WouldWrite => "would_write",
        StoreInitStepState::Exists => "exists",
        StoreInitStepState::Created => "created",
        StoreInitStepState::Written => "written",
        StoreInitStepState::Blocked => "blocked",
    }
}
