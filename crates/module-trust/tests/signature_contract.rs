use ed25519_dalek::{Signature, VerifyingKey};
use rz0_module_trust::{
    SignatureEnvelope, TrustedTestKey, canonical_message, verify_detached_signature,
};

fn envelope() -> SignatureEnvelope {
    serde_json::from_str(include_str!("fixtures/valid-envelope.json")).expect("valid envelope")
}

fn trusted_key() -> TrustedTestKey {
    serde_json::from_str(include_str!("fixtures/trusted-test-key.json")).expect("trusted test key")
}

#[test]
fn verifies_canonical_detached_signature_with_test_key() {
    let envelope = envelope();
    let report = verify_detached_signature(&envelope, &trusted_key());
    assert!(report.verified, "{:?}", report.errors);
    assert!(report.test_key_only);
    assert!(report.errors.is_empty());
    let expected = "runtime.zero.package-signature.v1\nscheme=ed25519\nkey_id=test.rfc8032.1\npackage_id=first-party.fixture\npackage_version=0.1.0\nmanifest_sha256=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n";
    assert_eq!(
        canonical_message(&envelope).expect("canonical message"),
        expected
    );
    let mut unsigned = envelope;
    unsigned.signature_hex.clear();
    assert_eq!(
        canonical_message(&unsigned).expect("unsigned message"),
        expected
    );
}

#[test]
fn rejects_identity_digest_signature_and_revocation_drift() {
    let key = trusted_key();

    let mut digest_drift = envelope();
    digest_drift.manifest_sha256.replace_range(0..1, "f");
    assert!(!verify_detached_signature(&digest_drift, &key).verified);

    let mut signature_drift = envelope();
    signature_drift.signature_hex.replace_range(0..1, "0");
    assert!(!verify_detached_signature(&signature_drift, &key).verified);

    let mut identity_drift = envelope();
    identity_drift.package_version = "0.1.1".to_string();
    assert!(!verify_detached_signature(&identity_drift, &key).verified);

    let mut revoked = key;
    revoked.revoked = true;
    let report = verify_detached_signature(&envelope(), &revoked);
    assert!(!report.verified);
    assert!(report.errors.iter().any(|error| error.contains("revoked")));
}

#[test]
fn rejects_unauthorized_or_malformed_metadata_before_crypto() {
    let mut key = trusted_key();
    key.allowed_package_ids = vec!["first-party.other".to_string()];
    let report = verify_detached_signature(&envelope(), &key);
    assert!(!report.verified);
    assert!(
        report
            .errors
            .iter()
            .any(|error| error.contains("not authorized"))
    );

    let mut malformed = envelope();
    malformed.key_id = "UPPERCASE".to_string();
    malformed.signature_hex = "AA".repeat(64);
    let report = verify_detached_signature(&malformed, &trusted_key());
    assert!(!report.verified);
    assert!(report.errors.len() >= 2);
}

#[test]
fn underlying_verifier_matches_rfc_8032_test_vector_one() {
    let public_key =
        decode::<32>("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a");
    let signature = decode::<64>(
        "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155\
         5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
    );
    let key = VerifyingKey::from_bytes(&public_key).expect("RFC public key");
    assert!(
        key.verify_strict(&[], &Signature::from_bytes(&signature))
            .is_ok()
    );
}

fn decode<const N: usize>(value: &str) -> [u8; N] {
    let value = value.replace([' ', '\n'], "");
    assert_eq!(value.len(), N * 2);
    let mut output = [0u8; N];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).expect("fixture hex");
    }
    output
}
