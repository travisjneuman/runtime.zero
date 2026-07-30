use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const FINDING_SCHEMA_VERSION: u16 = 1;
pub const FINDING_CONTRACT: &str = "classified_finding_report";
pub const MAX_FINDING_REPORT_BYTES: u64 = rz0_resource_contract::MAX_FINDING_REPORT_BYTES;
pub const MAX_FINDING_SOURCES: usize = rz0_resource_contract::MAX_FINDING_SOURCES;
pub const MAX_FINDINGS: usize = rz0_resource_contract::MAX_FINDINGS;
pub const MAX_FINDING_SOURCE_REFERENCES: usize =
    rz0_resource_contract::MAX_FINDING_SOURCE_REFERENCES;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FindingReport {
    pub schema_version: u16,
    pub contract: String,
    pub report_id: String,
    pub producer_module_id: String,
    pub platform: String,
    pub input_evidence_sha256: String,
    pub read_only: bool,
    pub writes_attempted: bool,
    pub action_authorized: bool,
    pub raw_paths_included: bool,
    pub sources: Vec<FindingSource>,
    pub findings: Vec<Finding>,
    pub summary: FindingSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FindingSource {
    pub id: String,
    pub status: FindingSourceStatus,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingSourceStatus {
    Ok,
    Partial,
    Unavailable,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub finding_id: String,
    pub category: FindingCategory,
    pub subject_reference: String,
    pub source_ids: Vec<String>,
    pub ownership: FindingOwnership,
    pub data_class: FindingDataClass,
    pub confidence: FindingConfidence,
    pub risk: FindingRisk,
    pub disposition: FindingDisposition,
    pub exact_evidence: Option<ExactFindingEvidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingCategory {
    UpdateCandidate,
    UninstallCandidate,
    LeftoverCandidate,
    CacheCandidate,
    IntegrityObservation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingOwnership {
    ManagerOwned,
    RuntimeOwned,
    SystemOwned,
    UserOwned,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingDataClass {
    PackageMetadata,
    ApplicationMetadata,
    CacheData,
    OrphanedData,
    ExecutableArtifact,
    Configuration,
    CredentialOrSession,
    BrowserProfile,
    ProjectWorkspace,
    Backup,
    UserContent,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingConfidence {
    ExactEvidence,
    Corroborated,
    Heuristic,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingRisk {
    Low,
    Medium,
    High,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingDisposition {
    ReportOnly,
    ManagerActionCandidate,
    QuarantineCandidate,
    Ignore,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExactFindingEvidence {
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FindingSummary {
    pub source_count: usize,
    pub finding_count: usize,
    pub report_only_count: usize,
    pub manager_action_candidate_count: usize,
    pub quarantine_candidate_count: usize,
    pub ignore_count: usize,
    pub blocked_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingValidation {
    pub valid: bool,
    pub errors: Vec<String>,
}

pub fn seal_finding_report(report: &mut FindingReport) -> Result<(), String> {
    report.summary = summarize_findings(report);
    report.report_id = expected_report_id(report)?;
    let validation = validate_finding_report(report);
    if validation.valid {
        Ok(())
    } else {
        Err(validation.errors.join("; "))
    }
}

pub fn validate_finding_report(report: &FindingReport) -> FindingValidation {
    let mut errors = Vec::new();
    if report.schema_version != FINDING_SCHEMA_VERSION {
        errors.push(format!("schema_version must be {FINDING_SCHEMA_VERSION}"));
    }
    if report.contract != FINDING_CONTRACT {
        errors.push(format!("contract must be {FINDING_CONTRACT}"));
    }
    if producer_category(report.producer_module_id.as_str()).is_none() {
        errors.push("producer_module_id is not a finding-producing schema-1 module".to_string());
    }
    if !rz0_validation_contract::valid_ascii_text(&report.platform, 32) {
        errors.push("platform is invalid".to_string());
    }
    if !rz0_validation_contract::valid_sha256(&report.input_evidence_sha256) {
        errors.push("input_evidence_sha256 is invalid".to_string());
    }
    if !report.read_only
        || report.writes_attempted
        || report.action_authorized
        || report.raw_paths_included
    {
        errors
            .push("finding reports must remain read-only, path-free, and unauthorized".to_string());
    }
    if report.sources.is_empty() || report.sources.len() > MAX_FINDING_SOURCES {
        errors.push(format!(
            "sources must contain 1..={MAX_FINDING_SOURCES} entries"
        ));
    }
    if report.findings.len() > MAX_FINDINGS {
        errors.push(format!("findings exceed {MAX_FINDINGS} entries"));
    }

    let mut source_ids = BTreeSet::new();
    for source in report.sources.iter().take(MAX_FINDING_SOURCES) {
        if !rz0_validation_contract::valid_ledger_id(&source.id, 100)
            || !source_ids.insert(source.id.as_str())
        {
            errors.push("source IDs must be valid and unique".to_string());
        }
        if !rz0_validation_contract::valid_sha256(&source.evidence_sha256) {
            errors.push("source evidence SHA-256 is invalid".to_string());
        }
    }
    if report
        .sources
        .windows(2)
        .any(|pair| pair[0].id >= pair[1].id)
    {
        errors.push("sources must be sorted by ID".to_string());
    }

    let mut finding_ids = BTreeSet::new();
    for finding in report.findings.iter().take(MAX_FINDINGS) {
        if !rz0_validation_contract::valid_ledger_id(&finding.finding_id, 120)
            || !finding_ids.insert(finding.finding_id.as_str())
        {
            errors.push("finding IDs must be valid and unique".to_string());
        }
        if !rz0_validation_contract::valid_evidence_reference(&finding.subject_reference, 120) {
            errors.push("finding subject_reference is invalid".to_string());
        }
        if finding.source_ids.is_empty()
            || finding.source_ids.len() > MAX_FINDING_SOURCE_REFERENCES
            || finding.source_ids.windows(2).any(|pair| pair[0] >= pair[1])
            || finding
                .source_ids
                .iter()
                .any(|source_id| !source_ids.contains(source_id.as_str()))
        {
            errors.push(
                "finding source IDs must be present, sorted, unique, and bounded".to_string(),
            );
        }
        if producer_category(report.producer_module_id.as_str()) != Some(finding.category) {
            errors.push("finding category does not match its producer module".to_string());
        }
        validate_finding_policy(finding, &mut errors);
    }
    if report
        .findings
        .windows(2)
        .any(|pair| pair[0].finding_id >= pair[1].finding_id)
    {
        errors.push("findings must be sorted by ID".to_string());
    }
    if report.summary != summarize_findings(report) {
        errors.push("finding summary does not match report contents".to_string());
    }
    match expected_report_id(report) {
        Ok(expected) if report.report_id != expected => {
            errors.push("report_id does not bind the canonical report".to_string());
        }
        Err(error) => errors.push(error),
        _ => {}
    }

    errors.sort();
    errors.dedup();
    FindingValidation {
        valid: errors.is_empty(),
        errors,
    }
}

pub fn decode_finding_report(bytes: &[u8]) -> Result<FindingReport, String> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_FINDING_REPORT_BYTES {
        return Err("finding report is empty or oversized".to_string());
    }
    let report: FindingReport =
        serde_json::from_slice(bytes).map_err(|error| format!("parse finding report: {error}"))?;
    let validation = validate_finding_report(&report);
    if validation.valid {
        Ok(report)
    } else {
        Err(validation.errors.join("; "))
    }
}

pub fn finding_json(report: &FindingReport) -> Result<String, String> {
    let validation = validate_finding_report(report);
    if !validation.valid {
        return Err(validation.errors.join("; "));
    }
    serde_json::to_string_pretty(report)
        .map(|json| format!("{json}\n"))
        .map_err(|error| format!("render finding report: {error}"))
}

pub fn summarize_findings(report: &FindingReport) -> FindingSummary {
    FindingSummary {
        source_count: report.sources.len(),
        finding_count: report.findings.len(),
        report_only_count: count_disposition(report, FindingDisposition::ReportOnly),
        manager_action_candidate_count: count_disposition(
            report,
            FindingDisposition::ManagerActionCandidate,
        ),
        quarantine_candidate_count: count_disposition(
            report,
            FindingDisposition::QuarantineCandidate,
        ),
        ignore_count: count_disposition(report, FindingDisposition::Ignore),
        blocked_count: count_disposition(report, FindingDisposition::Blocked),
    }
}

fn validate_finding_policy(finding: &Finding, errors: &mut Vec<String>) {
    let protected = matches!(
        finding.data_class,
        FindingDataClass::CredentialOrSession
            | FindingDataClass::BrowserProfile
            | FindingDataClass::ProjectWorkspace
            | FindingDataClass::Backup
            | FindingDataClass::UserContent
            | FindingDataClass::Unknown
    );
    if protected || finding.ownership == FindingOwnership::Unknown {
        if finding.risk != FindingRisk::Blocked
            || finding.disposition != FindingDisposition::Blocked
        {
            errors.push("protected or unknown findings must remain blocked".to_string());
        }
    } else if (finding.risk == FindingRisk::Blocked)
        != (finding.disposition == FindingDisposition::Blocked)
    {
        errors.push("blocked risk and disposition must agree".to_string());
    }

    match finding.disposition {
        FindingDisposition::ManagerActionCandidate => {
            if finding.ownership != FindingOwnership::ManagerOwned
                || !matches!(
                    finding.category,
                    FindingCategory::UpdateCandidate | FindingCategory::UninstallCandidate
                )
            {
                errors.push(
                    "manager action candidates require manager ownership and category".to_string(),
                );
            }
        }
        FindingDisposition::QuarantineCandidate => {
            if finding.ownership != FindingOwnership::RuntimeOwned
                || !matches!(
                    finding.category,
                    FindingCategory::LeftoverCandidate | FindingCategory::CacheCandidate
                )
                || finding.exact_evidence.is_none()
            {
                errors.push(
                    "quarantine candidates require runtime ownership, exact evidence, and category"
                        .to_string(),
                );
            }
        }
        FindingDisposition::Ignore
        | FindingDisposition::ReportOnly
        | FindingDisposition::Blocked => {}
    }
    if finding.category == FindingCategory::IntegrityObservation
        && !matches!(
            finding.disposition,
            FindingDisposition::ReportOnly | FindingDisposition::Blocked
        )
    {
        errors.push("integrity observations are report-only or blocked".to_string());
    }
    if finding.confidence == FindingConfidence::ExactEvidence && finding.exact_evidence.is_none() {
        errors.push("exact confidence requires exact evidence".to_string());
    }
    if let Some(evidence) = &finding.exact_evidence
        && (!rz0_validation_contract::valid_sha256(&evidence.sha256)
            || evidence.size_bytes > rz0_resource_contract::MAX_ARTIFACT_BYTES)
    {
        errors.push("exact finding evidence is invalid or oversized".to_string());
    }
}

fn producer_category(module_id: &str) -> Option<FindingCategory> {
    match module_id {
        "first-party.updater" => Some(FindingCategory::UpdateCandidate),
        "first-party.uninstall" => Some(FindingCategory::UninstallCandidate),
        "first-party.leftovers" => Some(FindingCategory::LeftoverCandidate),
        "first-party.cache" => Some(FindingCategory::CacheCandidate),
        "first-party.security-integrity" => Some(FindingCategory::IntegrityObservation),
        _ => None,
    }
}

fn count_disposition(report: &FindingReport, disposition: FindingDisposition) -> usize {
    report
        .findings
        .iter()
        .filter(|finding| finding.disposition == disposition)
        .count()
}

fn expected_report_id(report: &FindingReport) -> Result<String, String> {
    let mut canonical = report.clone();
    canonical.report_id.clear();
    let bytes = serde_json::to_vec(&canonical)
        .map_err(|error| format!("serialize finding report identity: {error}"))?;
    let mut digest = Sha256::new();
    digest.update(b"runtime.zero.classified-finding-report.v1\0");
    digest.update((bytes.len() as u64).to_be_bytes());
    digest.update(bytes);
    let sha256 = format!("{:x}", digest.finalize());
    Ok(format!("findings:{}", &sha256[..24]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn report() -> FindingReport {
        let mut report = FindingReport {
            schema_version: FINDING_SCHEMA_VERSION,
            contract: FINDING_CONTRACT.to_string(),
            report_id: String::new(),
            producer_module_id: "first-party.cache".to_string(),
            platform: "test".to_string(),
            input_evidence_sha256: A.to_string(),
            read_only: true,
            writes_attempted: false,
            action_authorized: false,
            raw_paths_included: false,
            sources: vec![FindingSource {
                id: "cache.fixture".to_string(),
                status: FindingSourceStatus::Ok,
                evidence_sha256: B.to_string(),
            }],
            findings: vec![Finding {
                finding_id: "cache.0001".to_string(),
                category: FindingCategory::CacheCandidate,
                subject_reference: "subject:cache-0001".to_string(),
                source_ids: vec!["cache.fixture".to_string()],
                ownership: FindingOwnership::RuntimeOwned,
                data_class: FindingDataClass::CacheData,
                confidence: FindingConfidence::ExactEvidence,
                risk: FindingRisk::Low,
                disposition: FindingDisposition::QuarantineCandidate,
                exact_evidence: Some(ExactFindingEvidence {
                    sha256: A.to_string(),
                    size_bytes: 12,
                }),
            }],
            summary: FindingSummary {
                source_count: 0,
                finding_count: 0,
                report_only_count: 0,
                manager_action_candidate_count: 0,
                quarantine_candidate_count: 0,
                ignore_count: 0,
                blocked_count: 0,
            },
        };
        seal_finding_report(&mut report).unwrap();
        report
    }

    #[test]
    fn exact_cache_finding_is_deterministic_private_and_non_authorizing() {
        let report = report();
        let validation = validate_finding_report(&report);
        assert!(validation.valid, "{:?}", validation.errors);
        assert_eq!(report.summary.quarantine_candidate_count, 1);
        assert!(!report.action_authorized);
        let json = finding_json(&report).unwrap();
        assert_eq!(decode_finding_report(json.as_bytes()).unwrap(), report);
        assert!(!json.contains("/private/"));
    }

    #[test]
    fn protected_unknown_and_cross_module_findings_fail_closed() {
        let mut report = report();
        report.findings[0].data_class = FindingDataClass::CredentialOrSession;
        report.findings[0].ownership = FindingOwnership::Unknown;
        report.findings[0].risk = FindingRisk::Low;
        report.findings[0].disposition = FindingDisposition::QuarantineCandidate;
        report.findings[0].category = FindingCategory::UninstallCandidate;
        report.summary = summarize_findings(&report);
        report.report_id = expected_report_id(&report).unwrap();
        let validation = validate_finding_report(&report);
        assert!(!validation.valid);
        assert!(validation.errors.len() >= 3);
    }

    #[test]
    fn fabricated_authority_summary_digest_and_unknown_fields_fail_closed() {
        let mut report = report();
        report.action_authorized = true;
        report.summary.finding_count = 99;
        report.input_evidence_sha256 = "invalid".to_string();
        assert!(!validate_finding_report(&report).valid);

        let json = finding_json(&super::tests::report()).unwrap();
        let drifted = json.replacen(
            "\"schema_version\": 1",
            "\"schema_version\": 1,\n  \"future\": true",
            1,
        );
        assert!(decode_finding_report(drifted.as_bytes()).is_err());
        assert!(decode_finding_report(&vec![b'x'; MAX_FINDING_REPORT_BYTES as usize + 1]).is_err());
    }
}
