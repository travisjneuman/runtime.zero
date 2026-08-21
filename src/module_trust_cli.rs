use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::Path;

use crate::module_manifest::ModuleManifest;
use crate::module_validation::{ManifestValidationReport, validate_manifest};
use crate::{ExitCode, brand};
use rz0_module_trust::{
    SignatureEnvelope, SignatureVerification, TrustedTestKey, verify_detached_signature,
};

const MAX_TRUST_DOCUMENT_BYTES: u64 = rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModuleTrustVerificationReport {
    pub manifest_path: String,
    pub signature_path: String,
    pub trusted_key_path: String,
    pub valid: bool,
    pub manifest_sha256: Option<String>,
    pub manifest_identity_matches_signature: bool,
    pub manifest_validation: ManifestValidationReport,
    pub signature_verification: Option<SignatureVerification>,
    pub test_key_only: bool,
    pub execution_authorized: bool,
    pub writes_attempted: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub safety_note: &'static str,
}

pub fn verify_module_trust(
    manifest_path: &Path,
    signature_path: &Path,
    trusted_key_path: &Path,
) -> ModuleTrustVerificationReport {
    let (manifest_validation, manifest_bytes, mut errors) =
        load_manifest_with_exact_bytes(manifest_path);
    let manifest_sha256 = manifest_bytes
        .as_deref()
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)));

    let envelope = match read_json::<SignatureEnvelope>(signature_path, "signature envelope") {
        Ok(envelope) => Some(envelope),
        Err(error) => {
            errors.push(error);
            None
        }
    };
    let trusted_key = match read_json::<TrustedTestKey>(trusted_key_path, "trusted test key") {
        Ok(key) => Some(key),
        Err(error) => {
            errors.push(error);
            None
        }
    };

    let signature_verification = envelope
        .as_ref()
        .zip(trusted_key.as_ref())
        .map(|(envelope, trusted_key)| verify_detached_signature(envelope, trusted_key));

    let manifest_identity_matches_signature = manifest_validation
        .manifest
        .as_ref()
        .zip(envelope.as_ref())
        .is_some_and(|(manifest, envelope)| {
            manifest.id == envelope.package_id
                && manifest.version == envelope.package_version
                && manifest_sha256.as_deref() == Some(envelope.manifest_sha256.as_str())
        });
    if envelope.is_some() && !manifest_identity_matches_signature {
        errors.push(
            "signature identity or manifest digest does not match the exact manifest bytes"
                .to_string(),
        );
    }

    let signature_valid = signature_verification
        .as_ref()
        .is_some_and(|verification| verification.verified);
    let valid = errors.is_empty()
        && manifest_validation.valid
        && manifest_identity_matches_signature
        && signature_valid;
    let warnings = manifest_validation.warnings.clone();

    ModuleTrustVerificationReport {
        manifest_path: manifest_path.display().to_string(),
        signature_path: signature_path.display().to_string(),
        trusted_key_path: trusted_key_path.display().to_string(),
        valid,
        manifest_sha256,
        manifest_identity_matches_signature,
        manifest_validation,
        signature_verification,
        test_key_only: true,
        execution_authorized: false,
        writes_attempted: false,
        errors,
        warnings,
        safety_note: "Read-only trust review only; no package, registry, store, process, or module lifecycle action was performed.",
    }
}

fn load_manifest_with_exact_bytes(
    path: &Path,
) -> (ManifestValidationReport, Option<Vec<u8>>, Vec<String>) {
    let bytes = match read_bounded_file(path, "manifest") {
        Ok(bytes) => bytes,
        Err(error) => {
            return (
                ManifestValidationReport {
                    path: path.display().to_string(),
                    valid: false,
                    manifest: None,
                    integrity: None,
                    errors: vec![error],
                    warnings: Vec::new(),
                },
                None,
                Vec::new(),
            );
        }
    };
    let manifest = match serde_json::from_slice::<ModuleManifest>(&bytes) {
        Ok(manifest) => manifest,
        Err(error) => {
            let message = format!("invalid manifest JSON: {error}");
            return (
                ManifestValidationReport {
                    path: path.display().to_string(),
                    valid: false,
                    manifest: None,
                    integrity: None,
                    errors: vec![message.clone()],
                    warnings: Vec::new(),
                },
                Some(bytes),
                vec![message],
            );
        }
    };
    (validate_manifest(path, manifest), Some(bytes), Vec::new())
}

fn read_json<T: DeserializeOwned>(path: &Path, label: &str) -> Result<T, String> {
    let bytes = read_bounded_file(path, label)?;
    serde_json::from_slice(&bytes).map_err(|error| format!("invalid {label} JSON: {error}"))
}

fn read_bounded_file(path: &Path, label: &str) -> Result<Vec<u8>, String> {
    if looks_url_like(&path.display().to_string()) {
        return Err(format!(
            "{label} URLs are not supported; select a local file"
        ));
    }
    let metadata =
        fs::metadata(path).map_err(|error| format!("failed to inspect {label}: {error}"))?;
    if !metadata.is_file() {
        return Err(format!("{label} path is not a file"));
    }
    if metadata.len() > MAX_TRUST_DOCUMENT_BYTES {
        return Err(format!("{label} exceeds {MAX_TRUST_DOCUMENT_BYTES} bytes"));
    }
    fs::read(path).map_err(|error| format!("failed to read {label}: {error}"))
}

fn looks_url_like(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("://")
        || lower.starts_with("file:")
        || lower.starts_with("http:")
        || lower.starts_with("https:")
}

pub fn trust_command(args: &[String]) -> (ExitCode, String, String) {
    let Ok(request) = parse_args(args) else {
        return (ExitCode::Usage, String::new(), trust_usage());
    };
    let report = verify_module_trust(
        Path::new(&request.manifest),
        Path::new(&request.signature),
        Path::new(&request.trusted_key),
    );
    let code = if report.valid {
        ExitCode::Ok
    } else {
        ExitCode::Usage
    };
    match request.format {
        OutputFormat::Text => (code, trust_text(&report), String::new()),
        OutputFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(json) => (code, format!("{json}\n"), String::new()),
            Err(error) => (ExitCode::Usage, String::new(), error.to_string()),
        },
    }
}

pub fn trust_usage() -> String {
    format!(
        "Usage: {} modules trust verify --manifest <manifest.json> --signature <envelope.json> --trusted-test-key <key.json> [--format text|json]\n\nSafety: local test-key verification is read-only and never authorizes installation, activation, invocation, or any module lifecycle action.\n",
        brand::COMMAND
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

struct TrustRequest {
    manifest: String,
    signature: String,
    trusted_key: String,
    format: OutputFormat,
}

fn parse_args(args: &[String]) -> Result<TrustRequest, ()> {
    if args.first().map(String::as_str) != Some("verify") {
        return Err(());
    }
    let mut manifest = None;
    let mut signature = None;
    let mut trusted_key = None;
    let mut format = OutputFormat::Text;
    let mut index = 1usize;
    while index < args.len() {
        match args[index].as_str() {
            "--manifest" => set_path(&mut manifest, args, &mut index)?,
            "--signature" => set_path(&mut signature, args, &mut index)?,
            "--trusted-test-key" => set_path(&mut trusted_key, args, &mut index)?,
            "--json" => format = OutputFormat::Json,
            "--format" => {
                let Some(value) = args.get(index + 1).map(String::as_str) else {
                    return Err(());
                };
                format = match value {
                    "text" => OutputFormat::Text,
                    "json" => OutputFormat::Json,
                    _ => return Err(()),
                };
                index += 1;
            }
            _ => return Err(()),
        }
        index += 1;
    }
    Ok(TrustRequest {
        manifest: manifest.ok_or(())?,
        signature: signature.ok_or(())?,
        trusted_key: trusted_key.ok_or(())?,
        format,
    })
}

fn set_path(slot: &mut Option<String>, args: &[String], index: &mut usize) -> Result<(), ()> {
    if slot.is_some() {
        return Err(());
    }
    let Some(value) = args.get(*index + 1) else {
        return Err(());
    };
    *slot = Some(value.clone());
    *index += 1;
    Ok(())
}

fn trust_text(report: &ModuleTrustVerificationReport) -> String {
    let status = if report.valid { "valid" } else { "invalid" };
    let signature_status = report
        .signature_verification
        .as_ref()
        .map(|verification| {
            if verification.verified {
                "verified"
            } else {
                "rejected"
            }
        })
        .unwrap_or("unavailable");
    let mut out = format!("{} module trust review\n\n", brand::TITLE);
    let _ = writeln!(&mut out, "status: {status}");
    let _ = writeln!(&mut out, "manifest: {}", report.manifest_path);
    let _ = writeln!(
        &mut out,
        "manifest_sha256: {}",
        report.manifest_sha256.as_deref().unwrap_or("unavailable")
    );
    let _ = writeln!(
        &mut out,
        "manifest_validation: {}",
        if report.manifest_validation.valid {
            "valid"
        } else {
            "invalid"
        }
    );
    let _ = writeln!(&mut out, "signature: {signature_status}");
    let _ = writeln!(
        &mut out,
        "identity_and_digest_match: {}",
        report.manifest_identity_matches_signature
    );
    let _ = writeln!(&mut out, "test_key_only: {}", report.test_key_only);
    let _ = writeln!(
        &mut out,
        "execution_authorized: {}",
        report.execution_authorized
    );
    let _ = writeln!(&mut out, "writes_attempted: {}", report.writes_attempted);
    for error in &report.errors {
        let _ = writeln!(&mut out, "error: {error}");
    }
    for warning in &report.warnings {
        let _ = writeln!(&mut out, "warning: {warning}");
    }
    let _ = writeln!(&mut out, "safety: {}", report.safety_note);
    out
}
