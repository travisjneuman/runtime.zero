mod model;
mod validation;

pub use model::{
    KeyPurpose, SIGNATURE_SCHEMA_VERSION, SIGNATURE_SCHEME, SignatureEnvelope,
    SignatureVerification, TrustedTestKey,
};
pub use validation::{canonical_message, verify_detached_signature};
