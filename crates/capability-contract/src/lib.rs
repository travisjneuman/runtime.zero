use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    ProcessEnvironmentRead,
    FilesystemMetadataRead,
    PersistedEnvironmentRegistryRead,
    ApplicationRegistryRead,
    ApplicationFilesystemRead,
    ExactCommandProbe,
    NetworkMetadata,
    ManagerExecution,
    ElevatedManagerAction,
    RuntimeStateWrite,
    QuarantineWrite,
    RestoreWrite,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityValidation {
    pub valid: bool,
    pub errors: Vec<&'static str>,
}

pub fn validate_schema_one_manifest_permissions(
    declared: &[Capability],
    default_grants: &[Capability],
    explicit_grants: &[Capability],
) -> CapabilityValidation {
    let mut errors = Vec::new();
    let declared_set = declared.iter().copied().collect::<BTreeSet<_>>();
    let default_set = default_grants.iter().copied().collect::<BTreeSet<_>>();
    let explicit_set = explicit_grants.iter().copied().collect::<BTreeSet<_>>();
    if declared_set.len() != declared.len()
        || default_set.len() != default_grants.len()
        || explicit_set.len() != explicit_grants.len()
        || declared.windows(2).any(|pair| pair[0] >= pair[1])
        || default_grants.windows(2).any(|pair| pair[0] >= pair[1])
        || explicit_grants.windows(2).any(|pair| pair[0] >= pair[1])
    {
        errors.push("permission lists must be unique and sorted");
    }
    if !default_set.is_disjoint(&explicit_set) {
        errors.push("permission cannot be both default and explicit");
    }
    if default_set
        .union(&explicit_set)
        .any(|permission| !declared_set.contains(permission))
    {
        errors.push("granted permissions must also appear in declared");
    }
    if declared_set
        .iter()
        .any(|permission| !default_set.contains(permission) && !explicit_set.contains(permission))
    {
        errors.push("each declared permission must be default or explicit");
    }
    if declared_set
        .iter()
        .any(|permission| !permission.is_schema1_manifest_permission())
    {
        errors.push("permission is outside read-only manifest schema 1");
    }
    if default_set
        .iter()
        .any(|permission| permission.requires_explicit_manifest_grant())
    {
        errors.push("sensitive reads must require explicit grants");
    }
    CapabilityValidation {
        valid: errors.is_empty(),
        errors,
    }
}

pub fn validate_schema_one_protocol_grants(
    grants: &[Capability],
    maximum: usize,
) -> CapabilityValidation {
    validate_schema_one_grants(grants, maximum, Capability::is_schema1_protocol_capability)
}

pub fn validate_schema_one_action_grants(
    grants: &[Capability],
    maximum: usize,
) -> CapabilityValidation {
    validate_schema_one_grants(grants, maximum, Capability::is_schema1_action_capability)
}

fn validate_schema_one_grants(
    grants: &[Capability],
    maximum: usize,
    allowed: fn(Capability) -> bool,
) -> CapabilityValidation {
    let mut errors = Vec::new();
    let set = grants.iter().copied().collect::<BTreeSet<_>>();
    if maximum == 0
        || grants.len() > maximum
        || set.len() != grants.len()
        || grants.windows(2).any(|pair| pair[0] >= pair[1])
    {
        errors.push("capability grants must be unique, sorted, and bounded");
    }
    if grants
        .iter()
        .copied()
        .any(|capability| !allowed(capability))
    {
        errors.push("capability grant is outside its schema family");
    }
    CapabilityValidation {
        valid: errors.is_empty(),
        errors,
    }
}

impl Capability {
    pub const fn is_schema1_manifest_permission(self) -> bool {
        matches!(
            self,
            Self::ProcessEnvironmentRead
                | Self::FilesystemMetadataRead
                | Self::PersistedEnvironmentRegistryRead
                | Self::ApplicationRegistryRead
                | Self::ApplicationFilesystemRead
                | Self::ExactCommandProbe
        )
    }

    pub const fn is_schema1_protocol_capability(self) -> bool {
        self.is_schema1_manifest_permission()
    }

    pub const fn is_schema1_action_capability(self) -> bool {
        matches!(
            self,
            Self::NetworkMetadata
                | Self::ManagerExecution
                | Self::ElevatedManagerAction
                | Self::RuntimeStateWrite
                | Self::QuarantineWrite
                | Self::RestoreWrite
        )
    }

    pub const fn requires_explicit_manifest_grant(self) -> bool {
        matches!(
            self,
            Self::ApplicationRegistryRead
                | Self::ApplicationFilesystemRead
                | Self::ExactCommandProbe
        )
    }

    pub const fn is_network(self) -> bool {
        matches!(self, Self::NetworkMetadata)
    }

    pub const fn is_mutating(self) -> bool {
        matches!(
            self,
            Self::ManagerExecution
                | Self::ElevatedManagerAction
                | Self::RuntimeStateWrite
                | Self::QuarantineWrite
                | Self::RestoreWrite
        )
    }

    pub const fn requires_elevation(self) -> bool {
        matches!(self, Self::ElevatedManagerAction)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Capability, validate_schema_one_action_grants, validate_schema_one_manifest_permissions,
        validate_schema_one_protocol_grants,
    };

    #[test]
    fn manifest_permissions_are_exact_sorted_and_non_escalating() {
        use Capability::{
            ApplicationRegistryRead, ExactCommandProbe, FilesystemMetadataRead,
            ProcessEnvironmentRead,
        };
        let valid = validate_schema_one_manifest_permissions(
            &[
                ProcessEnvironmentRead,
                FilesystemMetadataRead,
                ApplicationRegistryRead,
                ExactCommandProbe,
            ],
            &[ProcessEnvironmentRead, FilesystemMetadataRead],
            &[ApplicationRegistryRead, ExactCommandProbe],
        );
        assert!(valid.valid, "{:?}", valid.errors);

        for (declared, defaults, explicit) in [
            (
                vec![FilesystemMetadataRead, ProcessEnvironmentRead],
                vec![FilesystemMetadataRead],
                vec![ProcessEnvironmentRead],
            ),
            (
                vec![ApplicationRegistryRead],
                vec![ApplicationRegistryRead],
                vec![],
            ),
            (vec![ProcessEnvironmentRead], vec![], vec![]),
            (
                vec![ProcessEnvironmentRead],
                vec![ProcessEnvironmentRead],
                vec![ProcessEnvironmentRead],
            ),
        ] {
            assert!(
                !validate_schema_one_manifest_permissions(&declared, &defaults, &explicit).valid
            );
        }
    }

    #[test]
    fn protocol_and_action_grant_families_cannot_be_mixed() {
        let protocol = [
            Capability::ProcessEnvironmentRead,
            Capability::FilesystemMetadataRead,
        ];
        assert!(validate_schema_one_protocol_grants(&protocol, 16).valid);
        assert!(!validate_schema_one_action_grants(&protocol, 16).valid);
        let action = [Capability::NetworkMetadata, Capability::RuntimeStateWrite];
        assert!(validate_schema_one_action_grants(&action, 16).valid);
        assert!(!validate_schema_one_protocol_grants(&action, 16).valid);
        assert!(!validate_schema_one_protocol_grants(&protocol, 1).valid);
        assert!(!validate_schema_one_protocol_grants(&protocol, 0).valid);
    }

    #[test]
    fn capability_families_are_disjoint_and_explicit() {
        let all = [
            Capability::ProcessEnvironmentRead,
            Capability::FilesystemMetadataRead,
            Capability::PersistedEnvironmentRegistryRead,
            Capability::ApplicationRegistryRead,
            Capability::ApplicationFilesystemRead,
            Capability::ExactCommandProbe,
            Capability::NetworkMetadata,
            Capability::ManagerExecution,
            Capability::ElevatedManagerAction,
            Capability::RuntimeStateWrite,
            Capability::QuarantineWrite,
            Capability::RestoreWrite,
        ];
        for capability in all {
            assert_ne!(
                capability.is_schema1_manifest_permission(),
                capability.is_schema1_action_capability()
            );
        }
        assert!(Capability::ApplicationRegistryRead.requires_explicit_manifest_grant());
        assert!(Capability::NetworkMetadata.is_network());
        assert!(Capability::RuntimeStateWrite.is_mutating());
        assert!(Capability::ElevatedManagerAction.requires_elevation());
    }
}
