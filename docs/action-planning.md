# Action Planning Contract

Inventory evidence must become a reviewable plan before any updater, uninstall,
leftover, cleanup, quarantine, or restore module can mutate a system. This
contract defines that future boundary; current commands do not execute these
actions.

## Pipeline

1. **Evidence** — read-only source records with provenance, status, warnings, and
   confidence.
2. **Finding** — normalized installed tool/app/path state without an action.
3. **Candidate action** — a module proposes one narrow operation and names its
   manager/source, target, risk, capabilities, and prerequisites.
4. **Dry-run plan** — the core validates policy, resolves exact paths/commands,
   reports expected writes and rollback, and sets `would_write: false`.
5. **Explicit approval** — a user approves the exact plan, never a vague module
   category or future action.
6. **Transaction** — only a separately approved execution layer may act and
   produce a receipt.
7. **Verification** — re-inventory confirms outcome; mismatches stop for review.

Discovery must remain useful when planning or execution is unavailable.

## Plan fields

A future versioned plan should include:

- plan ID, schema version, creation time, expiry, host/platform class, and module
  identity;
- evidence source IDs and immutable evidence digests;
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

Integration-test helpers now exercise quarantine/restore only under a marked,
prefixed direct child of the OS temporary root. Tests prove verified-copy-before-
remove, durable fixture record creation, restore without consuming quarantine,
occupied-destination refusal, tamper/symlink rejection, and a failure after copy
that retains both source and verified copy. Test cleanup removes only that
isolated root.

There is intentionally no `rz0` action-plan command, manager adapter, staging
executor, production filesystem mover, or runtime mutation path. Real
package-manager commands and non-fixture filesystem mutation still require
separate approval.
