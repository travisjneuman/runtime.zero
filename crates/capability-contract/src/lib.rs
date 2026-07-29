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
    use super::Capability;

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
