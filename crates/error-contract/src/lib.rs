use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FoundationErrorCode {
    ExecutionNotAuthorized,
    InvalidContract,
    UnsupportedPlatform,
    UnsupportedOperation,
    CapabilityDenied,
    TrustVerificationFailed,
    ArtifactIdentityChanged,
    InputLimitExceeded,
    OutputLimitExceeded,
    TimedOut,
    Cancelled,
    Conflict,
    TransactionInvalid,
    RecoveryRequired,
    PermissionDenied,
    ResourceExhausted,
    IoUnavailable,
    InternalInvariant,
}

pub const ALL_ERROR_CODES: [FoundationErrorCode; 18] = [
    FoundationErrorCode::ExecutionNotAuthorized,
    FoundationErrorCode::InvalidContract,
    FoundationErrorCode::UnsupportedPlatform,
    FoundationErrorCode::UnsupportedOperation,
    FoundationErrorCode::CapabilityDenied,
    FoundationErrorCode::TrustVerificationFailed,
    FoundationErrorCode::ArtifactIdentityChanged,
    FoundationErrorCode::InputLimitExceeded,
    FoundationErrorCode::OutputLimitExceeded,
    FoundationErrorCode::TimedOut,
    FoundationErrorCode::Cancelled,
    FoundationErrorCode::Conflict,
    FoundationErrorCode::TransactionInvalid,
    FoundationErrorCode::RecoveryRequired,
    FoundationErrorCode::PermissionDenied,
    FoundationErrorCode::ResourceExhausted,
    FoundationErrorCode::IoUnavailable,
    FoundationErrorCode::InternalInvariant,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Policy,
    Contract,
    Platform,
    Capability,
    Trust,
    Identity,
    Resource,
    Cancellation,
    Conflict,
    Transaction,
    Permission,
    Io,
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryDisposition {
    Never,
    AfterInputChange,
    AfterEnvironmentChange,
    ManualRecovery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorSemantics {
    pub category: ErrorCategory,
    pub retry: RetryDisposition,
    pub automatic_retry_allowed: bool,
    pub detail_must_be_redacted: bool,
    pub safe_for_json: bool,
}

impl FoundationErrorCode {
    pub const fn semantics(self) -> ErrorSemantics {
        use ErrorCategory as Category;
        use FoundationErrorCode as Code;
        use RetryDisposition as Retry;

        let (category, retry) = match self {
            Code::ExecutionNotAuthorized => (Category::Policy, Retry::Never),
            Code::InvalidContract => (Category::Contract, Retry::AfterInputChange),
            Code::UnsupportedPlatform | Code::UnsupportedOperation => {
                (Category::Platform, Retry::Never)
            }
            Code::CapabilityDenied => (Category::Capability, Retry::AfterInputChange),
            Code::TrustVerificationFailed => (Category::Trust, Retry::AfterInputChange),
            Code::ArtifactIdentityChanged => (Category::Identity, Retry::ManualRecovery),
            Code::InputLimitExceeded | Code::OutputLimitExceeded => {
                (Category::Resource, Retry::AfterInputChange)
            }
            Code::TimedOut | Code::ResourceExhausted => {
                (Category::Resource, Retry::AfterEnvironmentChange)
            }
            Code::Cancelled => (Category::Cancellation, Retry::Never),
            Code::Conflict => (Category::Conflict, Retry::ManualRecovery),
            Code::TransactionInvalid | Code::RecoveryRequired => {
                (Category::Transaction, Retry::ManualRecovery)
            }
            Code::PermissionDenied => (Category::Permission, Retry::AfterEnvironmentChange),
            Code::IoUnavailable => (Category::Io, Retry::AfterEnvironmentChange),
            Code::InternalInvariant => (Category::Internal, Retry::Never),
        };
        ErrorSemantics {
            category,
            retry,
            automatic_retry_allowed: false,
            detail_must_be_redacted: true,
            safe_for_json: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn stable_codes_are_unique_snake_case_and_fail_closed() {
        let mut serialized = BTreeSet::new();
        for code in ALL_ERROR_CODES {
            let value = serde_json::to_string(&code).expect("serialize code");
            assert!(serialized.insert(value.clone()));
            assert!(value.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'"')
            }));
            let semantics = code.semantics();
            assert!(!semantics.automatic_retry_allowed);
            assert!(semantics.detail_must_be_redacted);
            assert!(semantics.safe_for_json);
        }
        assert_eq!(serialized.len(), ALL_ERROR_CODES.len());
        assert!(serde_json::from_str::<FoundationErrorCode>("\"future_error\"").is_err());
    }

    #[test]
    fn authorization_and_recovery_errors_are_never_retryable() {
        assert_eq!(
            FoundationErrorCode::ExecutionNotAuthorized
                .semantics()
                .retry,
            RetryDisposition::Never
        );
        assert_eq!(
            FoundationErrorCode::RecoveryRequired.semantics().retry,
            RetryDisposition::ManualRecovery
        );
    }
}
