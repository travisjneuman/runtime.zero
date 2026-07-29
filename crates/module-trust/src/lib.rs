mod model;
mod staging;
mod staging_path;
mod validation;

pub use model::{
    KeyPurpose, SIGNATURE_SCHEMA_VERSION, SIGNATURE_SCHEME, SignatureEnvelope,
    SignatureVerification, TrustedTestKey,
};
pub use staging::{
    MAX_STAGING_FILE_BYTES, MAX_STAGING_FILES, MAX_STAGING_TOTAL_BYTES, STAGING_CONTRACT,
    STAGING_SCHEMA_VERSION, StagingFile, StagingFileRole, StagingPlan, StagingPlanValidation,
    StagingRollback, StagingRootClass, StagingSignatureProof, validate_staging_plan,
    validate_staging_plan_with_signature,
};
pub use staging_path::validate_relative_path;
pub use validation::{canonical_message, verify_detached_signature};
