use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::{ExitCode, brand};

const RELEASE_STATUS_CONTRACT: &str = "release_status_report";
const RELEASE_STATUS_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ReleaseStatusReport {
    schema_version: u16,
    contract: String,
    assessment_id: String,
    scope_revision: String,
    decision: rz0_release_contract::ReleaseDecision,
    release_authorized: bool,
    summary: rz0_release_contract::AcceptanceSummary,
    validation: rz0_release_contract::ReleaseValidation,
}

pub fn release_command(args: &[String]) -> (ExitCode, String, String) {
    match args {
        [] => (ExitCode::Ok, usage(), String::new()),
        [help] if matches!(help.as_str(), "--help" | "-h" | "help") => {
            (ExitCode::Ok, usage(), String::new())
        }
        [status, rest @ ..] if status == "status" => status_command(rest),
        _ => (
            ExitCode::Usage,
            String::new(),
            format!("unsupported release option\n\n{}", usage()),
        ),
    }
}

fn status_command(args: &[String]) -> (ExitCode, String, String) {
    let mut assessment_path = None;
    let mut format = OutputFormat::Text;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--assessment" if assessment_path.is_none() => {
                let Some(path) = args.get(index + 1) else {
                    return status_usage_error();
                };
                assessment_path = Some(PathBuf::from(path));
                index += 1;
            }
            "--assessment" => return status_usage_error(),
            "--json" => format = OutputFormat::Json,
            "--format" => {
                let Some(value) = args.get(index + 1).map(String::as_str) else {
                    return status_usage_error();
                };
                format = match value {
                    "text" => OutputFormat::Text,
                    "json" => OutputFormat::Json,
                    _ => return status_usage_error(),
                };
                index += 1;
            }
            "--help" | "-h" | "help" => return (ExitCode::Ok, usage(), String::new()),
            _ => return status_usage_error(),
        }
        index += 1;
    }
    let Some(path) = assessment_path else {
        return status_usage_error();
    };
    let assessment = match read_assessment(&path) {
        Ok(assessment) => assessment,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("release assessment could not be read: {error}\n"),
            );
        }
    };
    let validation = rz0_release_contract::validate_release_assessment(&assessment);
    let report = ReleaseStatusReport {
        schema_version: RELEASE_STATUS_SCHEMA_VERSION,
        contract: RELEASE_STATUS_CONTRACT.to_string(),
        assessment_id: assessment.assessment_id.clone(),
        scope_revision: assessment.scope_revision.clone(),
        decision: assessment.decision,
        release_authorized: assessment.release_authorized,
        summary: rz0_release_contract::summarize_acceptance(&assessment),
        validation,
    };
    let code = if report.validation.valid {
        ExitCode::Ok
    } else {
        ExitCode::Usage
    };
    match format {
        OutputFormat::Text => (code, render_text(&report), String::new()),
        OutputFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(json) => (code, format!("{json}\n"), String::new()),
            Err(error) => (
                ExitCode::Usage,
                String::new(),
                format!("release status serialization failed: {error}\n"),
            ),
        },
    }
}

fn read_assessment(
    path: &Path,
) -> Result<rz0_release_contract::ReleaseAcceptanceAssessment, String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err("assessment path must be a regular file, not a link or directory".to_string());
    }
    if metadata.len() > rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES {
        return Err("assessment exceeds the bounded document limit".to_string());
    }
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    if bytes.len() as u64 > rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES {
        return Err("assessment grew beyond the bounded document limit".to_string());
    }
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

fn render_text(report: &ReleaseStatusReport) -> String {
    let summary = &report.summary;
    let mut output = format!(
        "{} release status\n\nassessment: {}\nscope: {}\ndecision: {}\nrelease authorized: {}\ntargets: {}\nacceptance cells: {}\n  missing: {}\n  proven: {}\n  not applicable: {}\nvalidation: {}\n",
        brand::TITLE,
        report.assessment_id,
        report.scope_revision,
        decision_name(report.decision),
        yes_no(report.release_authorized),
        summary.targets,
        summary.cells,
        summary.missing,
        summary.proven,
        summary.not_applicable,
        yes_no(report.validation.valid),
    );
    if report.validation.valid {
        output.push_str("\nrelease gate: blocked until the frozen target × module × lifecycle evidence is complete and independently accepted\n");
    } else {
        output.push_str("\nvalidation errors:\n");
        for error in &report.validation.errors {
            output.push_str("  - ");
            output.push_str(error);
            output.push('\n');
        }
    }
    output
}

fn decision_name(decision: rz0_release_contract::ReleaseDecision) -> &'static str {
    match decision {
        rz0_release_contract::ReleaseDecision::Blocked => "blocked",
    }
}

const fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn status_usage_error() -> (ExitCode, String, String) {
    (
        ExitCode::Usage,
        String::new(),
        format!("release status requires one assessment file\n\n{}", usage()),
    )
}

fn usage() -> String {
    format!(
        "Usage: {} release status --assessment <assessment.json> [--format text|json]\n\nReads one bounded, explicit release-acceptance assessment. It never discovers targets, authorizes publication, signs artifacts, or mutates state.\n",
        brand::COMMAND
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;
    use rz0_release_contract::{
        Architecture, ArtifactKind, PlatformFamily, ReleaseAcceptanceAssessment, ReleaseDecision,
        ReleaseTarget, SupportTier, missing_cells_for_targets,
    };

    #[test]
    fn status_reports_a_valid_but_blocked_assessment() {
        let path = fixture_path("valid");
        let assessment = assessment();
        fs::write(
            &path,
            serde_json::to_vec(&assessment).expect("assessment bytes"),
        )
        .expect("assessment fixture");
        let (code, output, error) = release_command(&[
            "status".to_string(),
            "--assessment".to_string(),
            path.display().to_string(),
            "--format".to_string(),
            "json".to_string(),
        ]);
        let _ = fs::remove_file(&path);
        assert_eq!(code, ExitCode::Ok, "{error}");
        assert!(error.is_empty());
        assert!(output.contains("\"release_authorized\": false"));
        assert!(output.contains("\"missing\": 84"));
    }

    #[test]
    fn invalid_assessment_is_visible_and_non_authorizing() {
        let path = fixture_path("invalid");
        let mut assessment = assessment();
        assessment.cells.pop();
        fs::write(
            &path,
            serde_json::to_vec(&assessment).expect("assessment bytes"),
        )
        .expect("assessment fixture");
        let (code, output, error) = release_command(&[
            "status".to_string(),
            "--assessment".to_string(),
            path.display().to_string(),
        ]);
        let _ = fs::remove_file(&path);
        assert_eq!(code, ExitCode::Usage);
        assert!(error.is_empty());
        assert!(output.contains("validation: no"));
        assert!(output.contains("exact"));
    }

    #[test]
    fn missing_assessment_is_a_usage_error() {
        let (code, output, error) = release_command(&["status".to_string()]);
        assert_eq!(code, ExitCode::Usage);
        assert!(output.is_empty());
        assert!(error.contains("requires one assessment"));
    }

    fn fixture_path(label: &str) -> PathBuf {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        std::env::temp_dir().join(format!(
            "rz0-release-status-{}-{}-{}.json",
            std::process::id(),
            label,
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn assessment() -> ReleaseAcceptanceAssessment {
        let targets = vec![ReleaseTarget {
            target_id: "macos-current-arm64-zip".to_string(),
            platform: PlatformFamily::Macos,
            generation: "current".to_string(),
            variant: "default".to_string(),
            architecture: Architecture::Arm64,
            artifact: ArtifactKind::PortableZip,
            tier: SupportTier::ReleaseBlocking,
            vendor_supported: true,
        }];
        ReleaseAcceptanceAssessment {
            schema_version: rz0_release_contract::RELEASE_SCHEMA_VERSION,
            contract: rz0_release_contract::RELEASE_CONTRACT.to_string(),
            assessment_id: "rz0release-test".to_string(),
            scope_revision: "scope-test".to_string(),
            decision: ReleaseDecision::Blocked,
            release_authorized: false,
            cells: missing_cells_for_targets(&targets),
            targets,
        }
    }
}
