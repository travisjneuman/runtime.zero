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
rollback, module lifecycle transition, or release. Action plans must separately
bind exact source evidence and pass all confirmation/transaction/runtime gates.

## Remaining work

The contract is implemented and fixture-tested, but no live updater, uninstall,
leftovers, cache, or integrity collector emits it yet. Domain adapters need
synthetic/adversarial fixtures first, then artifact-only Windows/macOS/Linux
runtime proof. Modules may narrow categories and limits but cannot loosen this
foundation policy.
