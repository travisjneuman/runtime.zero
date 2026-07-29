use std::collections::BTreeSet;
use std::fmt::Write;

use ed25519_dalek::{Signature, VerifyingKey};

use crate::model::{
    SIGNATURE_SCHEMA_VERSION, SIGNATURE_SCHEME, SignatureEnvelope, SignatureVerification,
    TrustedTestKey,
};

const MAX_ALLOWED_PACKAGE_IDS: usize = 128;
const SIGNATURE_DOMAIN: &str = "runtime.zero.package-signature.v1";

pub fn verify_detached_signature(
    envelope: &SignatureEnvelope,
    trusted_key: &TrustedTestKey,
) -> SignatureVerification {
    let mut report = SignatureVerification {
        schema_version: SIGNATURE_SCHEMA_VERSION,
        verified: false,
        test_key_only: true,
        scheme: envelope.scheme.clone(),
        key_id: envelope.key_id.clone(),
        package_id: envelope.package_id.clone(),
        package_version: envelope.package_version.clone(),
        manifest_sha256: envelope.manifest_sha256.clone(),
        errors: Vec::new(),
    };
    validate_envelope(envelope, &mut report.errors);
    validate_trusted_key(trusted_key, &mut report.errors);

    if envelope.key_id != trusted_key.key_id {
        report
            .errors
            .push("signature key ID does not match the selected trusted key".to_string());
    }
    if envelope.scheme != trusted_key.scheme {
        report
            .errors
            .push("signature scheme does not match the selected trusted key".to_string());
    }
    if !trusted_key
        .allowed_package_ids
        .iter()
        .any(|package_id| package_id == &envelope.package_id)
    {
        report
            .errors
            .push("trusted test key is not authorized for this package ID".to_string());
    }
    if !report.errors.is_empty() {
        return report;
    }

    let public_key = match decode_hex::<32>(&trusted_key.public_key_hex) {
        Ok(value) => value,
        Err(error) => {
            report.errors.push(error);
            return report;
        }
    };
    let signature = match decode_hex::<64>(&envelope.signature_hex) {
        Ok(value) => Signature::from_bytes(&value),
        Err(error) => {
            report.errors.push(error);
            return report;
        }
    };
    let verifying_key = match VerifyingKey::from_bytes(&public_key) {
        Ok(key) => key,
        Err(_) => {
            report
                .errors
                .push("trusted test public key is not a valid Ed25519 key".to_string());
            return report;
        }
    };
    let message = canonical_message_unchecked(envelope);
    if verifying_key
        .verify_strict(message.as_bytes(), &signature)
        .is_err()
    {
        report.errors.push(
            "detached signature did not verify for the canonical package identity".to_string(),
        );
        return report;
    }

    report.verified = true;
    report
}

pub fn canonical_message(envelope: &SignatureEnvelope) -> Result<String, String> {
    let mut errors = Vec::new();
    validate_envelope_identity(envelope, &mut errors);
    if errors.is_empty() {
        Ok(canonical_message_unchecked(envelope))
    } else {
        Err(errors.join("; "))
    }
}

fn canonical_message_unchecked(envelope: &SignatureEnvelope) -> String {
    let mut message = String::new();
    let _ = writeln!(message, "{SIGNATURE_DOMAIN}");
    let _ = writeln!(message, "scheme={}", envelope.scheme);
    let _ = writeln!(message, "key_id={}", envelope.key_id);
    let _ = writeln!(message, "package_id={}", envelope.package_id);
    let _ = writeln!(message, "package_version={}", envelope.package_version);
    let _ = writeln!(message, "manifest_sha256={}", envelope.manifest_sha256);
    message
}

fn validate_envelope(envelope: &SignatureEnvelope, errors: &mut Vec<String>) {
    validate_envelope_identity(envelope, errors);
    validate_lower_hex(&envelope.signature_hex, 128, "detached signature", errors);
}

fn validate_envelope_identity(envelope: &SignatureEnvelope, errors: &mut Vec<String>) {
    if envelope.schema_version != SIGNATURE_SCHEMA_VERSION {
        errors.push(format!(
            "signature schema_version must be {SIGNATURE_SCHEMA_VERSION}"
        ));
    }
    if envelope.scheme != SIGNATURE_SCHEME {
        errors.push(format!("signature scheme must be {SIGNATURE_SCHEME}"));
    }
    validate_id(&envelope.key_id, "signature key_id", errors);
    validate_id(&envelope.package_id, "signature package_id", errors);
    if !is_valid_version(&envelope.package_version) {
        errors.push(
            "signature package_version must use bounded ASCII version characters".to_string(),
        );
    }
    validate_lower_hex(
        &envelope.manifest_sha256,
        64,
        "signature manifest_sha256",
        errors,
    );
}

fn validate_trusted_key(key: &TrustedTestKey, errors: &mut Vec<String>) {
    if key.schema_version != SIGNATURE_SCHEMA_VERSION {
        errors.push(format!(
            "trusted key schema_version must be {SIGNATURE_SCHEMA_VERSION}"
        ));
    }
    if key.scheme != SIGNATURE_SCHEME {
        errors.push(format!("trusted key scheme must be {SIGNATURE_SCHEME}"));
    }
    validate_id(&key.key_id, "trusted key_id", errors);
    validate_lower_hex(&key.public_key_hex, 64, "trusted public key", errors);
    if key.revoked {
        errors.push("trusted test key is revoked".to_string());
    }
    if key.allowed_package_ids.is_empty() || key.allowed_package_ids.len() > MAX_ALLOWED_PACKAGE_IDS
    {
        errors.push(format!(
            "trusted key must authorize 1 to {MAX_ALLOWED_PACKAGE_IDS} package IDs"
        ));
    }
    let mut seen = BTreeSet::new();
    for package_id in &key.allowed_package_ids {
        validate_id(package_id, "trusted key package ID", errors);
        if !seen.insert(package_id) {
            errors.push("trusted key package IDs must not contain duplicates".to_string());
            break;
        }
    }
}

fn validate_id(value: &str, label: &str, errors: &mut Vec<String>) {
    let valid = !value.is_empty()
        && value.len() <= 80
        && !value.starts_with(['.', '-'])
        && !value.ends_with(['.', '-'])
        && !value.contains("..")
        && value.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '.' | '-')
        });
    if !valid {
        errors.push(format!(
            "{label} must use bounded lowercase letters, digits, dots, or hyphens"
        ));
    }
}

fn is_valid_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 40
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '+' | '-')
        })
}

fn validate_lower_hex(value: &str, length: usize, label: &str, errors: &mut Vec<String>) {
    if value.len() != length
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        errors.push(format!(
            "{label} must contain exactly {length} lowercase hexadecimal characters"
        ));
    }
}

fn decode_hex<const N: usize>(value: &str) -> Result<[u8; N], String> {
    if value.len() != N * 2 {
        return Err("hexadecimal value has an invalid length".to_string());
    }
    let mut output = [0u8; N];
    let bytes = value.as_bytes();
    for (index, output_byte) in output.iter_mut().enumerate() {
        let high = decode_nibble(bytes[index * 2])?;
        let low = decode_nibble(bytes[index * 2 + 1])?;
        *output_byte = (high << 4) | low;
    }
    Ok(output)
}

fn decode_nibble(value: u8) -> Result<u8, String> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _ => Err("hexadecimal value contains an invalid character".to_string()),
    }
}
