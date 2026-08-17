# runtime.zero Architecture

> Current implementation state, validation evidence, updater caveats, and the
> dependency-ordered restart sequence are captured in
> [`project-status-and-resumption.md`](project-status-and-resumption.md).

`runtime.zero` is a modular system-management runtime, not a monolithic cleaner script. The core is the smallest durable foundation that can describe, validate, list, and eventually run explicitly installed modules under safety policy.

## Layers

1. **CLI core** — argument parsing, launch routing, stable text/JSON output, exit codes, built-in inventory/monitor surfaces, and the explicit updater coordinator.
2. **Interactive TUI** — Ratatui widgets over Crossterm terminal lifecycle, one canonical software list, cached review controls, and CLI action handoffs.
3. **Module registry** — manifest model, local manifest validation, installed-module listing, and core-vs-module reporting.
4. **Policy and contracts** — safety posture, validation, resources, privacy, capabilities, errors, confirmation, cancellation, transactions, and release evidence.
5. **Action pipeline** — evidence, findings, dry-run plans, exact approval, transaction, and post-action verification. The updater consumes the first bounded core execution lane; other domains remain blocked.
6. **Platform adapters** — Windows, macOS, and Linux-specific discovery, monitoring, filesystem, process, manager, and future mutation primitives.
7. **Modules** — separately built domain packages that require explicit lifecycle/trust before core execution. The inventory library is embedded only as a bounded read adapter.
8. **Quarantine/restore** — test-proven semantics for future timestamped local quarantine instead of hard delete; no product mover exists yet.

## Foundation boundary

The core may include self-description, `doctor`, manifest schemas, output
contracts, policy primitives, bounded read-only inventory adapters, and the
explicitly scoped updater executor needed for a useful zero-module product. It
must not bundle arbitrary write-capable domain modules by default. First-party
modules should be optional packages with declared capabilities, risk level,
supported platforms, and safety behavior. Third-party modules require a
separate trust model before implementation.

Local manifest loading is read-only and declarative. Loading a manifest means
parsing and validating JSON metadata; it does not load code, fetch dependencies,
install anything, enable anything, or run module entry points.

## Initial platform intent

The originating use case was a Windows CLI/tool-manager workflow, but Windows,
macOS, and Linux are equal release requirements. Shared Rust contracts are
platform-neutral; only the narrowest adapter layer may differ.

## Inventory contract boundary

Core `rz0 scan --dry-run --format json` emits a live, path-redacted versioned
`inventory_report`; `rz0 apps` emits the path-free installed-software view used
by the TUI. The collector library remains a separate workspace package under
`modules/inventory/`, but it is now a deliberate built-in core dependency.

A small `crates/inventory-contract/` library owns strict owned serialization,
deserialization, cross-reference/summary validation, resource ceilings, and a
separate private-for-export gate so the feature package does not depend on the
CLI/TUI core or pull its terminal stack into the module binary. The module uses deterministic fixtures, bounded
process-PATH collection on
Windows/macOS/Linux, read-only persisted PATH and optional app registry reads on
Windows, bounded `.app`/Homebrew/MacPorts/Apple-receipt and XDG/dpkg/pacman
evidence on macOS/Linux, metadata-only launchd/systemd/Windows service records,
allowlisted direct executable discovery, opt-in Unix probes using shared
cleared-environment drains/deadlines/process groups (Windows fails closed),
report-local path redaction, and structured source events. It does not invoke
package managers/service controllers, execute desktop entries, or recursively
scan drives. Bundle versions come only from bounded direct `Info.plist` reads;
receipt and launchd metadata use separately bounded plist reads. See
[`inventory-schema.md`](inventory-schema.md).

`crates/capability-contract/` owns the shared read/action capability vocabulary
used by manifests, protocols, and plans while granting no authority.
`crates/cancellation-contract/` owns one atomic first-writer-wins cancellation
and monotonic-deadline primitive. `crates/module-lifecycle/` owns digest-bound
install/activate/invoke/repair/migrate/upgrade/deactivate/uninstall transition
plans and exact foundation gate sets. `crates/privacy-contract/` owns bounded,
report-local redaction without retaining raw sensitive strings.
`crates/configuration-contract/` owns immutable fail-closed schema-1 defaults;
`crates/diagnostics-contract/` binds their digest into the strict privacy-safe
`rz0 doctor` report. `crates/support-contract/` validates private inventory and
diagnostics inputs and emits only deterministic summary counts/statuses and
domain-separated digests; both `rz0 report` and the separate report/export
module call it without sharing authority. `crates/performance-contract/` owns
bounded final-artifact command budgets and non-authorizing measurements.
`crates/process-host/` owns bounded cancellable pipe transport, a serialized Unix
descriptor-audit/spawn boundary, process-group teardown, and test containment;
the updater consumes its bound mutating transport, but it is not an OS sandbox.
`crates/finding-contract/` owns path-free typed ownership/data-class/confidence/
risk/disposition evidence and conservative protected-data policy for five module
families; it cannot authorize an action. Updater and uninstall now turn selected
live evidence into shared findings/plans, while leftovers/cache/integrity remain
synthetic and no non-updater execution exists. `crates/confirmation-contract/` owns
exact short-lived interactive plan binding,
response digests, and single-use consumption evidence without execution
authority. `crates/error-contract/` owns stable machine codes and conservative privacy/retry
semantics; human messages are not policy. `crates/resource-contract/` owns
shared byte/record/timeout/process ceilings so modules can narrow but not expand
foundation budgets. `crates/validation-contract/` owns allocation-free lexical
validation for contract IDs, versions, hashes, references, and relative paths.
`crates/secure-fs/` owns held-directory-relative create/open/sync/lock/no-replace
and atomic-replace primitives: Unix runtime-tested operations plus compile-
checked NT root-relative Windows operations and owner/DACL inspection. Windows
store mutation remains blocked pending safe initial ACL creation and runtime
proof.
`crates/registry-contract/` owns canonical installed-state shape, ordering,
paths, serialization, and digests. `crates/release-contract/` owns the bounded
canonical target × module × lifecycle evidence-ledger shape and cannot authorize
release.
`crates/artifact-identity/` opens, bounds, hashes, identifies, revalidates, and
returns the same file handle without execution. Linux can expose the held
`/proc/self/fd` spawn identity and the core updater consumes it for direct native
ELF managers; macOS uses direct-path identity/digest revalidation and production
Windows binding remains a later gate. `crates/module-trust/` supplies a
test-key-only detached Ed25519 verification contract without adding a signer,
key store, installer, execution path, or production trust root. Schema-1
staging plans and OS-temp integration tests also exercise immutable publication
and quarantine/restore failure semantics without a production mover.
`crates/transaction-contract/` owns the bounded hash-chained transaction state
machine, exclusive immutable snapshot publication/recovery, exact confirmation-
aware commit-receipt binding, single-use consumption publication, atomic registry-
last coordination, canonical external-manager effect receipts, boundary-aware
cancellation classification, and non-authorizing commit/recovery assessment.
Production rollback and recovery
mutation remain gated.
`crates/module-protocol/` validates a read-only/offline
invocation preview and not-executed response with exact-path/digest metadata,
least-privilege grants, a cleared environment-name allowlist, and bounded I/O.
An explicit-feature integration lane launches only a Cargo-built test helper
from a guarded OS-temp receipt-like path to exercise framing, shared bounded
drains, Unix inheritable-descriptor refusal, Unix process-group teardown, and
compile-only Windows Job Object tree teardown. Linux and Windows builds hold the
verified executable lease through spawn; Windows handle auditing and production
macOS module spawning still fail closed. The updater has a separate bounded
core execution lane with macOS path revalidation. No core or inventory-module
launch exists outside that updater lane. Future module
execution, production signing, capability enforcement, durable transactions,
and distribution work is gated by
[`module-trust-and-execution.md`](module-trust-and-execution.md).
Update/uninstall/quarantine semantics are gated by
[`action-planning.md`](action-planning.md). Equal Windows/macOS/Linux release
requirements, the frozen module catalog, and the rule that shared stability,
security, transaction, observability, and efficiency behavior belongs in the
foundation are defined by
[`production-readiness.md`](production-readiness.md).

## Current authority and non-goals

Implemented read paths do not grant write authority. Manifest validation,
package hashes, signatures, findings, dry-run plans, confirmations, receipts,
execution assessments, and release ledgers remain evidence with only the narrow
authority explicitly assigned by their caller.

The current exceptions and blocks are:

- `store init --yes` may create only validated runtime.zero-owned user-local
  scaffolding on supported Unix paths;
- `updates --apply` may invoke one freshly planned allowlisted manager action
  after exact confirmation and local transaction evidence, but remains pre-alpha;
  Linux native-ELF identity binding, SIGINT cancellation, external-effect
  receipts, and recovery status exist, while macOS/Windows binding, OS isolation,
  full cancellation, rollback/recovery completion, and platform proof remain;
- no uninstall, cleanup, permanent deletion, module install/activation, repair,
  quarantine/restore, or arbitrary module execution;
- no malware-removal or unsupported security assurance claims;
- no remote module execution or third-party trust;
- no public direct-run bootstrap command before release verification is complete;
- no package publication, release workflow, recurring automation, or deployment
  mutation without separate approval.

## Current execution flows

### Read flow

```text
platform source -> bounded adapter -> strict inventory/monitor contract
                -> path-free/private view -> CLI JSON/text or TUI
```

Each source reports partial/unavailable state independently. Evidence does not
become an action merely because it appears in the catalog.

### Updater flow

```text
fresh manager probe -> updater finding report -> dry-run action plan
-> one selected action -> exact five-minute confirmation
-> executable binding before confirmation consumption
-> durable consumption + exact write-intent journal
-> bounded cancellable identity-bound manager process
-> fresh availability verification -> canonical external-effect receipt
-> final committed journal
```

This is the only current system-manager write flow. Linux direct native ELF
execution consumes the opened lease; macOS uses path identity/digest
revalidation; Windows fails closed. Read-only
recovery status reconciles receipt/journal state, but exact recovery completion,
native rollback, OS capability isolation, and disposable-host proof remain
production blockers.

### Future module flow

```text
verified immutable package -> production trust/provenance
-> explicit lifecycle install/activate -> capability-brokered isolated process
-> domain finding/plan -> confirmation/transaction/rollback -> verification
```

Schema 1 deliberately cannot authorize that flow.
