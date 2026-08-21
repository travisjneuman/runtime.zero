use super::model::{
    ActionDisposition, BoundedId, BoundedText, DetailField, EvidenceRef, EvidenceSource, Freshness,
    RecordKind, RecordStatus, RedactionState, ReviewBoundary, Route, RouteProjection, SearchTerms,
    UiModel, UiRecord, ViewState,
};
use crate::tui_dashboard::{TuiDashboard, TuiRow, TuiSection};

pub fn loading_model(generation: u64) -> UiModel {
    UiModel::loading(generation)
}

pub fn unavailable_model(generation: u64, reason: &str) -> UiModel {
    UiModel::unavailable(generation, public_text(reason).as_str())
}

pub fn model_from_dashboard(dashboard: &TuiDashboard, generation: u64) -> UiModel {
    let mut model = UiModel::loading(generation);
    let ready = ViewState::Ready { generation };
    let overview = records_for_section(
        dashboard,
        "overview",
        Route::Overview,
        RecordKind::Readiness,
    );
    let explore = dashboard
        .sections
        .iter()
        .flat_map(|section| records_from_section(section, Route::Explore, RecordKind::Inventory))
        .collect::<Vec<_>>();
    let review = dashboard
        .sections
        .iter()
        .flat_map(|section| {
            records_from_section(section, Route::Review, RecordKind::ActionReview)
                .into_iter()
                .filter(|record| {
                    record.status == RecordStatus::Plan
                        || record.status == RecordStatus::Warn
                        || record.status == RecordStatus::Blocked
                        || record.review_boundary.as_ref().is_some_and(|boundary| {
                            boundary.disposition != ActionDisposition::Unavailable
                        })
                })
        })
        .collect::<Vec<_>>();
    let activity = dashboard
        .sections
        .iter()
        .filter(|section| matches!(section.title, "system" | "diagnostics"))
        .flat_map(|section| records_from_section(section, Route::Activity, RecordKind::Activity))
        .collect::<Vec<_>>();
    let modules = dashboard
        .sections
        .iter()
        .filter(|section| section.title == "diagnostics")
        .flat_map(|section| records_from_section(section, Route::Modules, RecordKind::Module))
        .collect::<Vec<_>>();

    set_route(
        &mut model,
        Route::Overview,
        ready.clone(),
        "attention and next safe step",
        overview,
    );
    set_route(
        &mut model,
        Route::Explore,
        ready.clone(),
        "searchable local evidence",
        explore,
    );
    set_route(
        &mut model,
        Route::Review,
        ready.clone(),
        "plans, findings, and blocked boundaries",
        review,
    );
    set_route(
        &mut model,
        Route::Activity,
        ready.clone(),
        "running, receipt, and recovery evidence",
        activity,
    );
    set_route(
        &mut model,
        Route::Modules,
        ready,
        "module posture from foundation evidence",
        modules,
    );
    model.state = ViewState::Ready { generation };
    model.status = bounded(format!(
        "{} software · {} modules · review first",
        dashboard.installed_software_count, dashboard.installed_module_count
    ));
    model.job = super::model::JobState::Idle;
    model
        .validate()
        .expect("foundation adapter creates a valid UI model");
    model
}

fn set_route(
    model: &mut UiModel,
    route: Route,
    ready: ViewState,
    summary: &str,
    records: Vec<UiRecord>,
) {
    let state = if records.is_empty() {
        ViewState::Empty {
            generation: ready.generation(),
        }
    } else {
        ready
    };
    *model.route_mut(route) = RouteProjection {
        route,
        state,
        summary: bounded(summary.to_string()),
        records,
    };
}

fn records_for_section(
    dashboard: &TuiDashboard,
    title: &str,
    route: Route,
    kind: RecordKind,
) -> Vec<UiRecord> {
    dashboard
        .sections
        .iter()
        .find(|section| section.title == title)
        .map(|section| records_from_section(section, route, kind))
        .unwrap_or_default()
}

fn records_from_section(section: &TuiSection, route: Route, kind: RecordKind) -> Vec<UiRecord> {
    section
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| record_from_row(section, index, row, route, kind))
        .collect()
}

fn record_from_row(
    section: &TuiSection,
    index: usize,
    row: &TuiRow,
    route: Route,
    kind: RecordKind,
) -> UiRecord {
    let title = public_text(&row.value);
    let summary = row
        .preview
        .as_deref()
        .map(public_text)
        .unwrap_or_else(|| public_text(&section.summary));
    let status = record_status(row.label, row.tone);
    let reference_id = bounded_id(format!(
        "evidence/{}/{index}",
        route.title().to_ascii_lowercase()
    ));
    let module_id = bounded_id(format!("core.{}", section.title));
    let action_disposition = match status {
        RecordStatus::Plan => ActionDisposition::Reviewable,
        RecordStatus::Blocked | RecordStatus::Warn => ActionDisposition::Blocked,
        _ => ActionDisposition::Unavailable,
    };
    let review_boundary = Some(ReviewBoundary {
        reference_id: bounded_id(format!(
            "review-boundary/{}/{index}",
            route.title().to_ascii_lowercase()
        )),
        disposition: action_disposition,
        message: bounded(
            "Read-only review boundary. The foundation must provide an exact action plan before any confirmation or execution path exists.",
        ),
    });
    let mut details = Vec::new();
    if let Ok(field) = DetailField::text("source", section.title) {
        details.push(field);
    }
    if let Ok(field) = DetailField::text("state", row.label) {
        details.push(field);
    }
    if let Some(preview) = row.preview.as_deref() {
        if let Ok(field) = DetailField::text("evidence", public_text(preview)) {
            details.push(field);
        }
    }
    UiRecord {
        record_id: bounded_id(format!("{}/{index}", section.code)),
        module_id,
        kind,
        title: bounded(title),
        summary: bounded(summary),
        status,
        details: vec![super::model::UiDetailSection {
            title: bounded("Evidence"),
            fields: details,
        }],
        evidence: vec![EvidenceRef {
            source: source_for(kind),
            reference_id,
            freshness: Freshness::Fresh,
            redaction: RedactionState::PathRedacted,
        }],
        action_refs: Vec::new(),
        review_boundary: Some(review_boundary.expect("review boundary is always present")),
        search_terms: SearchTerms(vec![
            bounded(section.title),
            bounded(row.label),
            bounded(&row.value),
        ]),
    }
}

fn source_for(kind: RecordKind) -> EvidenceSource {
    match kind {
        RecordKind::Provider | RecordKind::ActionReview => EvidenceSource::ProviderReview,
        RecordKind::Recovery => EvidenceSource::RecoveryReview,
        RecordKind::Module => EvidenceSource::ModuleRegistry,
        RecordKind::System | RecordKind::Activity => EvidenceSource::SystemMonitor,
        RecordKind::Diagnostic => EvidenceSource::CliContract,
        RecordKind::Inventory => EvidenceSource::Inventory,
        RecordKind::Readiness => EvidenceSource::LocalSnapshot,
        RecordKind::Finding => EvidenceSource::LocalSnapshot,
    }
}

fn record_status(label: &str, tone: &str) -> RecordStatus {
    match label {
        "[OK]" => RecordStatus::Ok,
        "[INFO]" => RecordStatus::Info,
        "[PLAN]" => RecordStatus::Plan,
        "[DRY-RUN]" => RecordStatus::DryRun,
        "[BLOCKED]" => RecordStatus::Blocked,
        "[ERROR]" => RecordStatus::Error,
        "[WARN]" => RecordStatus::Warn,
        "[SKIP]" => RecordStatus::Muted,
        _ if tone == "safe" => RecordStatus::Ok,
        _ if tone == "warn" => RecordStatus::Warn,
        _ => RecordStatus::Observed,
    }
}

fn public_text(value: &str) -> String {
    if value.contains("/Users/")
        || value.contains("/private/")
        || value.contains("\\Users\\")
        || value.contains("/var/")
    {
        return BoundedText::redacted().into_string();
    }
    let value = value.replace(['\n', '\r', '\t'], " ");
    if value.chars().count() > super::model::MAX_UI_TEXT_CHARS {
        value
            .chars()
            .take(super::model::MAX_UI_TEXT_CHARS.saturating_sub(1))
            .chain(std::iter::once('…'))
            .collect()
    } else if value.is_empty() {
        "[no detail]".to_string()
    } else {
        value
    }
}

fn bounded(value: impl Into<String>) -> BoundedText {
    BoundedText::try_new(value).unwrap_or_else(|_| BoundedText::redacted())
}

fn bounded_id(value: impl Into<String>) -> BoundedId {
    let value = value.into().replace(' ', "-");
    BoundedId::try_new(value)
        .unwrap_or_else(|_| BoundedId::try_new("ui.invalid").expect("static id is valid"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui_dashboard;

    #[test]
    fn adapter_produces_all_stable_routes_without_raw_paths() {
        let model = model_from_dashboard(&tui_dashboard::dashboard(), 7);
        assert_eq!(model.state.label(), "ready");
        for route in Route::ALL {
            assert!(!model.route(route).summary.as_str().is_empty());
            for record in &model.route(route).records {
                assert!(!record.title.as_str().contains("/Users/"));
                assert_eq!(record.evidence[0].redaction, RedactionState::PathRedacted);
            }
        }
    }

    #[test]
    fn adapter_marks_action_like_records_as_review_boundaries_only() {
        let model = model_from_dashboard(&tui_dashboard::dashboard(), 1);
        assert!(
            model
                .routes
                .iter()
                .flat_map(|route| route.records.iter())
                .any(|record| record.review_boundary.is_some())
        );
        assert!(
            model
                .routes
                .iter()
                .flat_map(|route| route.records.iter())
                .all(|record| record.action_refs.is_empty())
        );
    }
}
