use serde::{Deserialize, Serialize};

pub const SIGNATURE_SCHEMA_VERSION: u16 = 1;
pub const SIGNATURE_SCHEME: &str = "ed25519";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SignatureEnvelope {
    pub schema_version: u16,
    pub scheme: String,
    pub key_id: String,
    pub package_id: String,
    pub package_version: String,
    pub manifest_sha256: String,
    pub signature_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedTestKey {
    pub schema_version: u16,
    pub scheme: String,
    pub key_id: String,
    pub purpose: KeyPurpose,
    pub public_key_hex: String,
    pub allowed_package_ids: Vec<String>,
    pub revoked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyPurpose {
    TestOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SignatureVerification {
    pub schema_version: u16,
    pub verified: bool,
    pub test_key_only: bool,
    pub scheme: String,
    pub key_id: String,
    pub package_id: String,
    pub package_version: String,
    pub manifest_sha256: String,
    pub errors: Vec<String>,
}
