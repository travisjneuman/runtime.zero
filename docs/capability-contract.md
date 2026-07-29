# Shared Capability Contract

`crates/capability-contract/` owns the foundation capability vocabulary used by
module manifests, read-only process previews, and action plans. Modules do not
define private capability enums or reinterpret the same string differently.

The current vocabulary is:

| Capability | Schema-1 family | Meaning |
| --- | --- | --- |
| `process_environment_read` | Manifest/protocol | Read the explicitly provided process environment |
| `filesystem_metadata_read` | Manifest/protocol | Read bounded filesystem metadata |
| `persisted_environment_registry_read` | Manifest/protocol | Read persisted environment registry/configuration evidence |
| `application_registry_read` | Manifest/protocol, explicit only | Read bounded application registry evidence |
| `application_filesystem_read` | Manifest/protocol, explicit only | Read bounded application filesystem evidence |
| `exact_command_probe` | Manifest/protocol, explicit only | Run an exact allowlisted read-only probe |
| `network_metadata` | Action plan | Read remote availability metadata after network review |
| `manager_execution` | Action plan | Invoke an exact manager-native operation after later authorization |
| `elevated_manager_action` | Action plan | Invoke an exact manager operation requiring elevation |
| `runtime_state_write` | Action plan | Write receipt-listed runtime.zero-owned state |
| `quarantine_write` | Action plan | Write a receipt-listed quarantine payload/record |
| `restore_write` | Action plan | Restore a verified receipt-listed payload |

The enum exposes classification helpers for manifest/protocol/action schemas,
explicit grants, network, mutation, and elevation. Each owning validator still
enforces its narrower schema:

- manifest permission schema 1 rejects action/mutation capabilities;
- module protocol schema 1 rejects every non-read capability;
- action-plan schema 1 rejects read-only protocol permissions and enforces exact
  network/elevation/manager requirements;
- sensitive application/probe reads can never be default manifest grants.

This crate is vocabulary and classification only. It does not grant authority,
open files, contact a network, elevate, invoke a manager, write state, or execute
a module. A valid declared capability is not an OS-enforced capability. The
production execution assessment remains blocked until a platform broker proves
that every granted capability is enforced and all ungranted capabilities are
denied.

Future capabilities must be added here first with a versioned schema decision,
least-privilege semantics, platform enforcement design, TUI/text/JSON review
copy, deny behavior, and fixtures. Unknown serialized variants continue to fail
deserialization.

See [`manifest-validation.md`](manifest-validation.md),
[`action-planning.md`](action-planning.md),
[`module-process-protocol.md`](module-process-protocol.md), and
[`production-readiness.md`](production-readiness.md).
