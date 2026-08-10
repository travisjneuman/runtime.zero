# Action Planning Contract

Inventory evidence must become a reviewable plan before any updater, uninstall,
leftover, cleanup, quarantine, or restore module can mutate a system. This
contract defines the shared boundary; the updater's explicit `--apply` lane is
the first production-shaped mutation consumer, while uninstall, cleanup, and
module lifecycle consumers remain gated.

## Pipeline

1. **Evidence** — read-only source records with provenance, status, warnings, and
   confidence.
2. **Finding** — `crates/finding-contract/` validates path-free typed ownership,
   data class, confidence, risk, disposition, protected-data policy, evidence
   digests, and producer/category binding without granting action authority.
3. **Candidate action** — a module proposes one narrow operation and names its
   manager/source, target, risk, capabilities, and prerequisites.
4. **Dry-run plan** — the core validates policy, resolves exact paths/commands,
   reports expected writes and rollback, and sets `would_write: false`.
5. **Explicit approval** — the foundation binds the exact plan/dry-run/write-set/
   state digests into a short-lived interactive challenge; a user never approves
   a vague module category or future action.
6. **Transaction** — only a separately approved execution layer may act and
   produce a receipt.
7. **Verification** — re-inventory confirms outcome; mismatches stop for review.

Discovery must remain useful when planning or execution is unavailable.

## Plan fields

A future versioned plan should include:

- plan ID, schema version, creation time, expiry, host/platform class, and module
  identity;
- exact `classified_finding_report` contract, sealed report ID, immutable report
  digest, and one mandatory finding ID per action;
- action kind, exact target identity/version, manager/source, and rationale;
- for quarantine/restore fixtures, exact source-relative path, SHA-256, and size;
- capability grant and whether elevation/network access is required;
- exact command executable/arguments or exact runtime.zero-owned write set;
- risk category and blocked-data classifications;
- expected before/after state;
- dry-run/no-write fields;
- confirmation scope;
- quarantine, receipt, rollback, and post-action verification steps;
- warnings, unsupported conditions, and plan-invalidating drift checks.

Plans must not embed secrets, tokens, cookies, raw credentials, or private
session data.

## Update planning

Updater modules must:

- consider only already installed tools unless the user separately requests an
  install workflow;
- prefer the recorded/native package manager;
- separate local installed evidence from remote availability evidence;
- report when checking availability requires network access;
- pin the exact proposed target version/artifact when possible;
- never reinterpret a missing tool as permission to install it;
- keep self-update and module-update policy separate from system-tool updates.

## Uninstall planning

Uninstall modules must:

- use manager-native uninstall mechanisms first;
- identify the exact manager record/product/package and command;
- avoid direct deletion while a valid native uninstall path exists;
- enumerate known shared components and dependent packages when evidence exists;
- treat silent/unattended flags as separate reviewed behavior;
- stop before executing uninstallers, scripts, MSI products, package-manager
  commands, services, tasks, or elevated actions.

## Leftovers and cleanup

Post-uninstall findings must be classified before any action:

| Category | Default posture |
| --- | --- |
| Disposable cache proven to belong only to the target | Eligible for future quarantine plan |
| Stale shim/link with verified missing target | Eligible for future quarantine plan |
| Package-manager metadata | Manager-specific review |
| Config/state | Report only until ownership and user value are known |
| Logs | Report only; retention value may exceed space value |
| Credentials/session/browser profile | Blocked |
| Project/workspace/source data | Blocked |
| Backup/archive | Blocked |
| Shared/unknown data | Blocked |

No recursive drive sweep should become an action source. Findings need bounded
roots, ownership evidence, age/size metadata, and a reason.

## Quarantine and restore

Quarantine must precede deletion for eligible runtime.zero-owned actions:

- move or copy only receipt-listed paths into a plan-specific user-local area;
- record original path, metadata, digest, size, reason, timestamp, and restore
  conflict policy;
- verify the quarantined copy before removing the original;
- never overwrite an occupied restore destination silently;
- preserve a user-controlled retention decision;
- make permanent deletion a later explicit action, not a side effect of scan,
  uninstall, update, restore, or startup.

Cross-filesystem moves, permissions/ACLs, symlinks/reparse points, locked files,
and partial failures require platform-specific tests before mutation.

## Current implementation boundary

`crates/action-plan/` now supplies the schema-1 fixture-only model and fail-closed
validator. It uses the shared foundation capability vocabulary while rejecting
read-only protocol permissions; network, elevation, and manager grants must
match the exact plan flags/kind. Synthetic fixtures cover update, uninstall,
eligible quarantine,
restore, blocked credential/session classes, and invalid
write/confirmation/executable/forbidden-path combinations. Quarantine and
restore plans now bind an exact simulation-relative source path, lowercase
SHA-256, and bounded size; transaction-shape validation requires the matching
capabilities and write-set kinds. Every valid fixture is dry-run-only with
`writes_attempted: false` and every action has `would_write: false`.
Validated plans can now produce domain-separated deterministic plan and write-set
digests for confirmation, transaction, and receipt binding; invalid plans cannot.

Integration-test helpers now exercise quarantine/restore only under a marked,
prefixed direct child of the OS temporary root. Tests prove verified-copy-before-
remove, durable fixture record creation, restore without consuming quarantine,
occupied-destination refusal, tamper/symlink rejection, and a failure after copy
that retains both source and verified copy. Test cleanup removes only that
isolated root.

`crates/confirmation-contract/` adds exact short-lived CLI/TUI challenge,
response, and single-use consumption evidence while remaining structurally
unable to authorize execution.

The product-level `rz0 uninstall plan <installed-software-id>` command currently
emits a path-free `uninstall_review`. It is a UX bridge from live catalog
records, not a `crates/action-plan` value: it has no write set, cannot be
confirmed into execution, and always sets `product_execution_authorized: false`.
Before mutation, a review must be replaced or bound by exact finding-report and
shared action-plan evidence rather than becoming a second action system.

The updater now has a bounded, explicit manager-execution lane for live
platform probes: `rz0 updates --apply` requires an exact action ID (or the
interactive serial `--all` queue), explicit network-write approval, an
initialized private store, an exact short-lived challenge, and a manual-recovery
acknowledgement when manager rollback is not proven. It publishes a hash-chained
transaction journal and single-use confirmation before invoking the direct
allowlisted manager path, captures bounded output, verifies fresh manager
availability evidence, and writes a redacted receipt. It never invokes `sudo`
or an interactive privilege helper; elevated managers require an already-root
process. A failed command or verification leaves `recovery_required` evidence.

This does not authorize arbitrary module execution, third-party packages,
uninstall, cleanup, or filesystem mutation. Windows process containment and
manager-specific runtime proof remain fail-closed, and production use still
requires the platform/disposable-host evidence applicable to the target.

The updater execution bridge is not the final generic action executor. Its
allowlisted manager path is not yet bound through the opened-artifact lease into
spawn; network/capability flags are policy acknowledgements rather than an OS
sandbox; cancellation and rollback are incomplete; and its updater-specific
receipt publication does not yet use the complete canonical commit-receipt/
registry-last reconciliation path. A successful locally confirmed update report
therefore records one lane invocation, not platform, module, or release
readiness. These gaps must be closed before another domain copies the bridge.
