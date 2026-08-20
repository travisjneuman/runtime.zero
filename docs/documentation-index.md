# Documentation Guide

This guide is the map and precedence rule for `runtime.zero` documentation.
The repository is pre-alpha and contains both current product contracts and
historical validation snapshots. Read documents according to their purpose
instead of treating every dated statement as current runtime evidence.

## Documentation precedence

When two documents appear to conflict, use this order:

1. [`SAFETY.md`](../SAFETY.md) and [`SECURITY.md`](../SECURITY.md) for safety and
   security boundaries.
2. [`engineering-handoff.md`](engineering-handoff.md) for the product end state,
   modularity rules, and the next-shift direction.
3. [`project-status-and-resumption.md`](project-status-and-resumption.md) for the
   current implemented product, validation baseline, limitations, and next work.
4. [`production-readiness.md`](production-readiness.md) for the definition of a
   complete 1.0 product.
5. [`roadmap.md`](roadmap.md) for dependency order and checked/unchecked work.
6. The narrow topic contract for field-level or subsystem behavior.
7. Dated audits and old artifact measurements only for the source commit they
   name.
8. Private `_meta.notes` records for historical operator evidence; public
   behavior must still be verified against this repository.

Source and tests remain the final implementation evidence. A checked roadmap
item means a contract or bounded implementation exists; it does not by itself
mean production support, release approval, or completion of every platform
cell.

## Start here

| Document | Audience | Purpose |
| --- | --- | --- |
| [`README.md`](../README.md) | Users and contributors | Product overview, quick start, current command surface, and honest maturity summary |
| [`engineering-handoff.md`](engineering-handoff.md) | Next engineering shift | Full-system-management end state, module contract, enable/disable semantics, delivery waves, and current handoff |
| [`project-status-and-resumption.md`](project-status-and-resumption.md) | Maintainers | Current code baseline, capabilities, known debts, validation, and restart order |
| [`roadmap.md`](roadmap.md) | Maintainers and reviewers | Dependency-ordered implementation progress |
| [`production-readiness.md`](production-readiness.md) | Release and security reviewers | Finite 1.0 completion definition and platform × module × lifecycle matrix |
| [`completion-checklist.md`](completion-checklist.md) | Maintainers and release reviewers | Consolidated bullet-level list of every remaining 1.0 workstream |
| [`SAFETY.md`](../SAFETY.md) | Everyone | Non-negotiable read, write, confirmation, quarantine, and data-protection rules |
| [`SECURITY.md`](../SECURITY.md) | Reporters and maintainers | Vulnerability reporting, support posture, and security boundaries |
| [`CONTRIBUTING.md`](../CONTRIBUTING.md) | Contributors | Contribution scope, validation commands, and review expectations |

## User and operator guides

| Document | Purpose |
| --- | --- |
| [`user-guide.md`](user-guide.md) | Current workflows, Rust/AI toolchain snapshot, updater apply lane, TUI, uninstall review, recovery status, store, modules, and completion usage |
| [`troubleshooting.md`](troubleshooting.md) | Fail-closed triage for terminals, inventory, reports, updates, store, builds, and safe bug reports |
| [`recovery-guide.md`](recovery-guide.md) | Transaction/receipt states, cancellation outcomes, evidence preservation, and current recovery limits |
| [`privacy-and-sharing.md`](privacy-and-sharing.md) | Output sensitivity, redaction limits, network posture, support-export review, and retention caveats |
| [`platform-notes.md`](platform-notes.md) | macOS/Linux/Windows capability matrix, discovery depth, mutation blocks, and evidence expectations |

## Product and architecture

| Document | Current role |
| --- | --- |
| [`architecture.md`](architecture.md) | Core/module/platform layering, end-state control plane, current read and write flows, and authority boundaries |
| [`module-system.md`](module-system.md) | Module catalog, registry, manifests, enablement target, and core-versus-module ownership |
| [`foundation-readiness.md`](foundation-readiness.md) | Current foundation maturity and prerequisites for broader module work |
| [`tui.md`](tui.md) | Interactive routing, keys, layouts, rendering, accessibility, and TUI limitations |
| [`tui-redesign.md`](tui-redesign.md) | Current task-first TUI product and acceptance contract |
| [`system-monitor.md`](system-monitor.md) | Native monitor schema, platform collectors, metric caveats, and no-remediation boundary |
| [`inventory-schema.md`](inventory-schema.md) | Inventory report, collectors, privacy, validation, and remaining parity work |
| [`action-planning.md`](action-planning.md) | Evidence-to-finding-to-plan-to-confirmation-to-transaction pipeline |
| [`store-and-routing-contract.md`](store-and-routing-contract.md) | User-local state paths, store inspection/initialization, registry/receipt parsing, and launch routing |
| [`website-tui-parity-backlog.md`](website-tui-parity-backlog.md) | Deferred public-site alignment with the real terminal TUI |

## Shared foundation contracts

These documents describe implemented libraries. Unless explicitly stated,
validation is not authority to execute, mutate, recover, share externally, or
release.

| Document | Owning package / concern |
| --- | --- |
| [`validation-contract.md`](validation-contract.md) | `crates/validation-contract`: canonical IDs, versions, hashes, references, and relative paths |
| [`resource-contract.md`](resource-contract.md) | `crates/resource-contract`: shared byte, record, timeout, and process ceilings |
| [`error-contract.md`](error-contract.md) | `crates/error-contract`: stable machine errors, privacy, and no-auto-retry policy |
| [`capability-contract.md`](capability-contract.md) | `crates/capability-contract`: shared capability vocabulary and schema partitions |
| [`configuration-contract.md`](configuration-contract.md) | `crates/configuration-contract`: immutable default-deny schema-1 configuration |
| [`privacy-contract.md`](privacy-contract.md) | `crates/privacy-contract`: sensitive classes and report-local redaction |
| [`diagnostics-contract.md`](diagnostics-contract.md) | `crates/diagnostics-contract`: private, config-bound `doctor` reports |
| [`finding-contract.md`](finding-contract.md) | `crates/finding-contract`: path-free ownership/risk/disposition evidence |
| [`confirmation-contract.md`](confirmation-contract.md) | `crates/confirmation-contract`: exact five-minute, single-use confirmation |
| [`cancellation-contract.md`](cancellation-contract.md) | `crates/cancellation-contract`: first-reason cancellation and monotonic deadlines |
| [`process-host-foundation.md`](process-host-foundation.md) | `crates/process-host`: bounded direct process transport and containment limits |
| [`secure-filesystem.md`](secure-filesystem.md) | `crates/secure-fs`: opened-directory-relative state operations |
| [`artifact-identity.md`](artifact-identity.md) | `crates/artifact-identity`: same-open-handle digest/identity and partial spawn binding |
| [`installed-registry-contract.md`](installed-registry-contract.md) | `crates/registry-contract`: canonical installed-module state |
| [`transaction-journal.md`](transaction-journal.md) | `crates/transaction-contract`: journal, receipts, registry-last commit, and recovery assessment |
| [`module-lifecycle-contract.md`](module-lifecycle-contract.md) | `crates/module-lifecycle`: eight planning-only lifecycle transitions |
| [`performance-contract.md`](performance-contract.md) | `crates/performance-contract`: final-artifact command budgets and evidence |
| [`release-acceptance.md`](release-acceptance.md) | `crates/release-contract`: target × module × lifecycle evidence ledger |
| [`support-report-contract.md`](support-report-contract.md) | `crates/support-contract`: privacy-reviewed local support summaries |

## Module trust and execution

| Document | Purpose |
| --- | --- |
| [`manifest-validation.md`](manifest-validation.md) | Local manifest, permission, and package-integrity validation |
| [`signature-verification.md`](signature-verification.md) | Public-test-key detached Ed25519 verification and immutable staging simulation |
| [`module-process-protocol.md`](module-process-protocol.md) | Unauthorized module preview, test-child transport, and blocked production assessment |
| [`module-trust-and-execution.md`](module-trust-and-execution.md) | Complete trust, provenance, isolation, lifecycle, and revocation gate |
| [`transaction-simulation.md`](transaction-simulation.md) | Guarded test-only staging, quarantine, and restore semantics |

## Feature-module documentation

| Document | Current maturity |
| --- | --- |
| [`modules/inventory/README.md`](../modules/inventory/README.md) | Built-in read-only collector library plus separate development binary; lifecycle package remains planned |
| [`modules/updater/README.md`](../modules/updater/README.md) | Live/captured availability parsing, finding/action-plan/queue logic, and core-owned explicit apply lane |
| [`modules/uninstall/README.md`](../modules/uninstall/README.md) | Shared synthetic/live installed-software finding contract plus non-executing core review/action-plan surface; no uninstall execution |
| [`modules/leftovers/README.md`](../modules/leftovers/README.md) | Synthetic exact-runtime-owned classifier only |
| [`modules/cache/README.md`](../modules/cache/README.md) | Bounded known-root read-only cache evidence plus ownership-aware classifier; no cleanup |
| [`modules/security-integrity/README.md`](../modules/security-integrity/README.md) | Synthetic exact-digest observation classifier only |
| [`modules/report-export/README.md`](../modules/report-export/README.md) | Separate stdin/stdout summary development binary; not integrated into core lifecycle |
| [`domain-classifier-modules.md`](domain-classifier-modules.md) | Shared summary of the five classifier packages |

## Platform, release, and operations

| Document | Purpose |
| --- | --- |
| [`support-policy.md`](support-policy.md) | Rolling support tiers, target admission, managers, terminals, filesystems, and artifact-only host rule |
| [`windows-compatibility.md`](windows-compatibility.md) | Explicit Windows client/server/edition/architecture/shell/terminal scope |
| [`platform-notes.md`](platform-notes.md) | Current cross-platform behavior and mutation limitations |
| [`release-packaging.md`](release-packaging.md) | Local non-publishing ZIP/DMG and metadata builders |
| [`free-release-distribution.md`](free-release-distribution.md) | No-paid-signing distribution policy and honest unsigned-artifact posture |
| [`local-install.md`](local-install.md) | Windows developer-only local install/uninstall scripts |
| [`dependency-and-validation-audit.md`](dependency-and-validation-audit.md) | Dated dependency, license, and validation evidence; not a live support promise |

## Brand and public site

| Document | Purpose |
| --- | --- |
| [`BRAND.md`](../BRAND.md) | Canonical Dossier Navy / Burnished Brass visual and language system |
| [`assets/brand/README.md`](../assets/brand/README.md) | Candidate asset provenance and promotion rules |
| [`docs/brand/README.md`](brand/README.md) | Brand implementation-note index |
| [`site/README.md`](../site/README.md) | Static-site source/deployment context and known parity debt |

The connected site is a public surface, but the website mock is not the product
contract. The real CLI/TUI and current repository docs take precedence. Site
source changes may deploy and therefore remain a separately reviewed external
write.

## Historical and fixture notes

README files under crate test fixtures describe synthetic data and guarded test
roots only. They are not end-user documentation:

- `crates/action-plan/tests/fixtures/transaction/README.md`;
- `crates/module-protocol/tests/fixtures/README.md`;
- `crates/module-trust/tests/fixtures/README.md`;
- `crates/module-trust/tests/fixtures/staging/README.md`.

Private `_meta.notes/Projects/runtime.zero/` implementation logs preserve why a
change was made and what passed at that time. Historical notes are intentionally
not rewritten to impersonate current evidence. The canonical private project
hub and latest documentation audit identify which older plans are superseded.

## Documentation maintenance checklist

For every behavior-changing change:

1. Update the narrow contract and its module/package README.
2. Update `engineering-handoff.md` when the product direction, module contract,
   lifecycle semantics, or next-shift dependency changes.
3. Update `project-status-and-resumption.md` when the user-visible surface,
   validation baseline, or top remaining dependency changes.
4. Update `roadmap.md` only when implementation evidence changes a checkbox.
5. Update `production-readiness.md` only when the completion definition,
   required matrix, or maturity evidence changes.
6. Keep `README.md`, `SAFETY.md`, CLI help, TUI labels, and website claims
   mutually honest.
7. Preserve dated artifact/test measurements with their source commit; never
   silently relabel old evidence as current.
8. Run Markdown-link, formatting, test, privacy, and secret checks before
   publication.
