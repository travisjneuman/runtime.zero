//! Allocation-free validation for untrusted contract scalars and relative paths.
//!
//! This crate owns lexical policy only. Callers retain semantic policy such as
//! allowed path prefixes, reserved IDs, and schema-specific length ceilings.

pub const MODULE_ID_MAX_BYTES: usize = 80;
pub const VERSION_MAX_BYTES: usize = 40;
pub const RELATIVE_PATH_MAX_BYTES: usize = 1024;
pub const SHA256_HEX_BYTES: usize = 64;

/// Lowercase ASCII identifier using digits, dots, and hyphens.
pub fn valid_dotted_id(value: &str, maximum: usize) -> bool {
    valid_id_edges(value, maximum)
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
        })
}

/// Lowercase ASCII ledger identifier that additionally permits underscores.
pub fn valid_ledger_id(value: &str, maximum: usize) -> bool {
    valid_id_edges(value, maximum)
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
        })
}

pub fn valid_module_id(value: &str) -> bool {
    valid_dotted_id(value, MODULE_ID_MAX_BYTES)
}

pub fn valid_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= VERSION_MAX_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'+' | b'-'))
}

pub fn valid_lower_hex(value: &str, expected_bytes: usize) -> bool {
    value.len() == expected_bytes
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub fn valid_sha256(value: &str) -> bool {
    valid_lower_hex(value, SHA256_HEX_BYTES)
}

/// A platform-neutral, normalized relative path with forward-slash separators.
///
/// Drive prefixes, URI schemes, backslashes, empty/dot/parent components, and
/// control characters are rejected before any platform path API sees the value.
pub fn valid_relative_path(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && !value.contains(['\\', ':'])
        && !value.chars().any(char::is_control)
        && value
            .split('/')
            .all(|component| !component.is_empty() && !matches!(component, "." | ".."))
}

pub fn valid_contract_relative_path(value: &str) -> bool {
    valid_relative_path(value, RELATIVE_PATH_MAX_BYTES)
}

pub fn valid_ascii_text(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty()
        && value.len() <= maximum
        && value.is_ascii()
        && !value.chars().any(char::is_control)
}

/// Evidence reference grammar: lowercase ASCII ID plus `_` and `:`.
pub fn valid_evidence_reference(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && !value.starts_with(['.', '-', '_', ':'])
        && !value.ends_with(['.', '-', '_', ':'])
        && !value.contains("..")
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'-' | b'_' | b':')
        })
}

pub fn is_absolute_local_path(value: &str) -> bool {
    value.starts_with(['/', '\\'])
        || (value.len() >= 3
            && value.as_bytes()[1] == b':'
            && matches!(value.as_bytes()[2], b'\\' | b'/'))
}

fn valid_id_edges(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && !value.starts_with(['.', '-', '_'])
        && !value.ends_with(['.', '-', '_'])
        && !value.contains("..")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dotted_and_ledger_ids_are_bounded_and_distinct() {
        for value in ["first-party.inventory", "rz0plan-001", "a0"] {
            assert!(valid_dotted_id(value, 100));
        }
        for value in ["", ".hidden", "trailing-", "two..dots", "Upper", "a_b"] {
            assert!(!valid_dotted_id(value, 100), "accepted {value:?}");
        }
        assert!(valid_ledger_id("scope_2026-07-29", 100));
        assert!(!valid_ledger_id("scope/escape", 100));
        assert!(!valid_dotted_id(&"a".repeat(101), 100));
    }

    #[test]
    fn versions_and_digests_are_canonical() {
        assert!(valid_version("1.0.0-rc.1+portable"));
        assert!(!valid_version("1.0.0 rc1"));
        assert!(!valid_version(&"1".repeat(VERSION_MAX_BYTES + 1)));
        assert!(valid_sha256(&"0a".repeat(32)));
        assert!(!valid_sha256(&"0A".repeat(32)));
        assert!(!valid_sha256("00"));
    }

    #[test]
    fn relative_paths_reject_cross_platform_escape_grammars() {
        assert!(valid_contract_relative_path(
            "modules/first-party.inventory/0.1.0/rz0-module.json"
        ));
        for value in [
            "",
            "/absolute",
            "C:/windows",
            "C:\\windows",
            "../escape",
            "a/../escape",
            "a/./file",
            "a//file",
            "http://host/file",
            "a\\file",
            "a\0file",
        ] {
            assert!(!valid_contract_relative_path(value), "accepted {value:?}");
        }
        assert!(!valid_relative_path(
            &"a".repeat(RELATIVE_PATH_MAX_BYTES + 1),
            RELATIVE_PATH_MAX_BYTES
        ));
    }

    #[test]
    fn text_references_and_absolute_paths_are_conservative() {
        assert!(valid_ascii_text("native runtime proof", 32));
        assert!(!valid_ascii_text("contains\ncontrol", 32));
        assert!(!valid_ascii_text("non-ascii-é", 32));
        assert!(valid_evidence_reference("evidence:windows_11.x64-001", 120));
        assert!(!valid_evidence_reference("Evidence.001", 120));
        assert!(is_absolute_local_path("/tmp/file"));
        assert!(is_absolute_local_path("C:\\Temp\\file"));
        assert!(!is_absolute_local_path("modules/file"));
    }
}
