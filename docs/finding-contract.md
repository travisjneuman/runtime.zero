# Shared Finding Classification Contract

`crates/finding-contract/` owns the privacy-safe boundary between discovery and
action planning for updater, uninstall, leftovers, cache, and
security/integrity modules. Modules classify domain evidence; they may not
invent private ownership, protected-data, disposition, digest, or summary
semantics.

## Schema

A `classified_finding_report` binds:

- one exact first-party producer module and its allowed finding category;
- a platform class and canonical input-evidence SHA-256;
- 1–64 sorted evidence sources with individual status/digest;
- up to 4,096 sorted findings;
- exact disposition summary counts;
- a domain-separated deterministic `report_id` over the complete report.

Findings contain only a stable `subject_reference`, sorted source IDs, typed
ownership/data class/confidence/risk/disposition, and optional exact SHA-256/
size evidence. Raw paths are forbidden. The document is limited to 4 MiB,
strict-deserialized, unknown-field rejecting, and validated before rendering.

## Conservative policy

Schema 1 enforces these cross-module rules:

- credentials/sessions, browser profiles, project workspaces, backups, user
  content, unknown data classes, and unknown ownership are always blocked;
- manager-action candidates require manager ownership and update/uninstall
  categories;
- quarantine candidates require runtime ownership, leftover/cache category, and
  exact digest/size evidence;
- integrity observations are report-only or blocked;
- exact-confidence findings require exact evidence;
- producer and category must match exactly.

`read_only` is true while `writes_attempted`, `action_authorized`, and
`raw_paths_included` are false. A valid finding is evidence, not an action. It
cannot authorize a manager command, quarantine, confirmation, transaction,
rollback, module lifecycle transition, or release. Schema-1 action plans must
name the exact finding contract, sealed report ID, immutable report SHA-256, and
one finding ID per action, then separately bind source evidence and pass all
confirmation/transaction/runtime gates.

## Remaining work

The contract is implemented and fixture-tested. The updater now emits it from
strict fixture, captured-manager, and explicit live-probe evidence before
building action plans. Core uninstall reviews emit shared findings, and the
cache and leftovers now emit bounded live evidence through `rz0 cache --dry-run`
and `rz0 leftovers --dry-run`; cache evidence includes explicit review-age,
size-budget, lock-marker, and exclusion policy without granting cleanup
authority. Security/integrity has fixture and bounded exact-file review through
`rz0 integrity --dry-run` and remains report-only without a trusted baseline.

Every remaining domain needs comprehensive adversarial fixtures, bounded live
adapters, exact source provenance, CLI/JSON/TUI integration, and artifact-only
Windows/macOS/Linux runtime proof. The updater also needs production process,
transaction, rollback, cancellation, and platform hardening; producing a valid
finding does not satisfy those later gates. Modules may narrow categories and
limits but cannot loosen foundation policy.
