use std::{collections::BTreeMap, fmt};

use rz0_error_contract::FoundationErrorCode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const PRIVACY_SCHEMA_VERSION: u16 = 1;
pub const DEFAULT_MAX_REDACTION_TOKENS: usize = rz0_resource_contract::MAX_REDACTION_TOKENS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SensitiveValueClass {
    LocalPath,
    UserIdentity,
    HostIdentity,
    EnvironmentValue,
    RegistryLocation,
    ProcessOutput,
    CommandArgument,
}

impl SensitiveValueClass {
    const fn token_name(self) -> &'static str {
        match self {
            Self::LocalPath => "path",
            Self::UserIdentity => "user",
            Self::HostIdentity => "host",
            Self::EnvironmentValue => "environment",
            Self::RegistryLocation => "registry",
            Self::ProcessOutput => "process-output",
            Self::CommandArgument => "argument",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyDisposition {
    PublicMetadata,
    ReportLocalRedactionRequired,
    NeverCollect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PrivacyRule {
    pub class: SensitiveValueClass,
    pub default_disposition: PrivacyDisposition,
}

pub const SCHEMA_ONE_RULES: [PrivacyRule; 7] = [
    rule(
        SensitiveValueClass::LocalPath,
        PrivacyDisposition::ReportLocalRedactionRequired,
    ),
    rule(
        SensitiveValueClass::UserIdentity,
        PrivacyDisposition::NeverCollect,
    ),
    rule(
        SensitiveValueClass::HostIdentity,
        PrivacyDisposition::NeverCollect,
    ),
    rule(
        SensitiveValueClass::EnvironmentValue,
        PrivacyDisposition::ReportLocalRedactionRequired,
    ),
    rule(
        SensitiveValueClass::RegistryLocation,
        PrivacyDisposition::ReportLocalRedactionRequired,
    ),
    rule(
        SensitiveValueClass::ProcessOutput,
        PrivacyDisposition::ReportLocalRedactionRequired,
    ),
    rule(
        SensitiveValueClass::CommandArgument,
        PrivacyDisposition::ReportLocalRedactionRequired,
    ),
];

const fn rule(class: SensitiveValueClass, default_disposition: PrivacyDisposition) -> PrivacyRule {
    PrivacyRule {
        class,
        default_disposition,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivacyErrorCode {
    EmptyValue,
    LimitExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivacyError {
    pub code: PrivacyErrorCode,
    detail: &'static str,
}

impl PrivacyError {
    pub const fn foundation_code(&self) -> FoundationErrorCode {
        match self.code {
            PrivacyErrorCode::EmptyValue => FoundationErrorCode::InvalidContract,
            PrivacyErrorCode::LimitExceeded => FoundationErrorCode::InputLimitExceeded,
        }
    }
}

impl fmt::Display for PrivacyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.detail)
    }
}

impl std::error::Error for PrivacyError {}

/// Stable report-local redaction without retaining raw sensitive strings.
///
/// The map key is a domain-separated SHA-256 digest of class plus value. Tokens
/// are assigned by first encounter, so callers must traverse canonical report
/// order when deterministic bytes are required.
#[derive(Debug)]
pub struct RedactionContext {
    tokens: BTreeMap<[u8; 32], u32>,
    maximum_tokens: usize,
}

impl Default for RedactionContext {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_REDACTION_TOKENS).expect("default redaction ceiling is positive")
    }
}

impl RedactionContext {
    pub fn new(maximum_tokens: usize) -> Result<Self, PrivacyError> {
        if maximum_tokens == 0 || maximum_tokens > DEFAULT_MAX_REDACTION_TOKENS {
            return Err(PrivacyError {
                code: PrivacyErrorCode::LimitExceeded,
                detail: "redaction token ceiling is outside the foundation limit",
            });
        }
        Ok(Self {
            tokens: BTreeMap::new(),
            maximum_tokens,
        })
    }

    pub fn redact(
        &mut self,
        class: SensitiveValueClass,
        value: &str,
    ) -> Result<String, PrivacyError> {
        if value.is_empty() {
            return Err(PrivacyError {
                code: PrivacyErrorCode::EmptyValue,
                detail: "sensitive value must not be empty",
            });
        }
        let key = redaction_key(class, value);
        let sequence = if let Some(sequence) = self.tokens.get(&key) {
            *sequence
        } else {
            if self.tokens.len() >= self.maximum_tokens {
                return Err(PrivacyError {
                    code: PrivacyErrorCode::LimitExceeded,
                    detail: "report exceeded its redaction token ceiling",
                });
            }
            let sequence = u32::try_from(self.tokens.len() + 1).map_err(|_| PrivacyError {
                code: PrivacyErrorCode::LimitExceeded,
                detail: "redaction token sequence overflowed",
            })?;
            self.tokens.insert(key, sequence);
            sequence
        };
        Ok(format!("<redacted:{}:{sequence:04}>", class.token_name()))
    }

    pub fn redact_optional(
        &mut self,
        class: SensitiveValueClass,
        value: &mut Option<String>,
    ) -> Result<(), PrivacyError> {
        if let Some(raw) = value.as_deref() {
            *value = Some(self.redact(class, raw)?);
        }
        Ok(())
    }

    pub fn token_count(&self) -> usize {
        self.tokens.len()
    }

    pub fn maximum_tokens(&self) -> usize {
        self.maximum_tokens
    }
}

fn redaction_key(class: SensitiveValueClass, value: &str) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(b"runtime.zero.report-local-redaction.v1\0");
    let class_name = class.token_name();
    digest.update((class_name.len() as u64).to_be_bytes());
    digest.update(class_name.as_bytes());
    digest.update((value.len() as u64).to_be_bytes());
    digest.update(value.as_bytes());
    digest.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_values_receive_stable_report_local_tokens() {
        let mut context = RedactionContext::new(3).unwrap();
        let first = context
            .redact(SensitiveValueClass::LocalPath, "/private/example")
            .unwrap();
        let duplicate = context
            .redact(SensitiveValueClass::LocalPath, "/private/example")
            .unwrap();
        let second = context
            .redact(SensitiveValueClass::LocalPath, "/private/other")
            .unwrap();
        assert_eq!(first, "<redacted:path:0001>");
        assert_eq!(duplicate, first);
        assert_eq!(second, "<redacted:path:0002>");
        assert_eq!(context.token_count(), 2);
        assert!(!format!("{context:?}").contains("/private/example"));
    }

    #[test]
    fn value_classes_are_domain_separated() {
        let mut context = RedactionContext::new(2).unwrap();
        let path = context
            .redact(SensitiveValueClass::LocalPath, "same")
            .unwrap();
        let argument = context
            .redact(SensitiveValueClass::CommandArgument, "same")
            .unwrap();
        assert_eq!(path, "<redacted:path:0001>");
        assert_eq!(argument, "<redacted:argument:0002>");
    }

    #[test]
    fn empty_and_over_ceiling_values_fail_closed() {
        let mut context = RedactionContext::new(1).unwrap();
        assert_eq!(
            context
                .redact(SensitiveValueClass::LocalPath, "")
                .unwrap_err()
                .foundation_code(),
            FoundationErrorCode::InvalidContract
        );
        context
            .redact(SensitiveValueClass::LocalPath, "first")
            .unwrap();
        assert_eq!(
            context
                .redact(SensitiveValueClass::LocalPath, "second")
                .unwrap_err()
                .foundation_code(),
            FoundationErrorCode::InputLimitExceeded
        );
        assert!(RedactionContext::new(0).is_err());
        assert!(RedactionContext::new(DEFAULT_MAX_REDACTION_TOKENS + 1).is_err());
    }

    #[test]
    fn schema_rules_cover_each_class_once_and_never_publish_sensitive_values() {
        let classes = SCHEMA_ONE_RULES
            .iter()
            .map(|rule| rule.class)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(classes.len(), SCHEMA_ONE_RULES.len());
        assert!(
            SCHEMA_ONE_RULES
                .iter()
                .all(|rule| { rule.default_disposition != PrivacyDisposition::PublicMetadata })
        );
    }
}
