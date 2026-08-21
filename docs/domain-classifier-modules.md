# Domain Finding Classifier Modules

Five first-party packages consume the shared
`crates/finding-contract/`, but their maturity is no longer identical.

## Current package boundaries

| Package | Current source behavior | Live/core status |
| --- | --- | --- |
| `modules/updater/` | Classifies installed/manager-owned update evidence, parses selected manager output, builds finding-bound action plans and serial queues | Fixture, captured-output, and explicit live-probe paths exist; core owns the separate explicit apply lane |
| `modules/uninstall/` | Classifies synthetic or live installed-software evidence and builds optional finding-bound manager plans | Core `uninstall plan` uses this shared producer; the foundation owns the narrow exact-manager apply boundary |
| `modules/leftovers/` | Classifies bounded runtime.zero-owned module/log and unreferenced-receipt evidence for conservative post-uninstall review | Core `rz0 leftovers --dry-run` live adapter, strict fixture path, exact module-file plan, and confirmation-bound exact quarantine; no recursive cleanup |
| `modules/cache/` | Classifies bounded known-root cache evidence while preserving conservative ownership policy | Core `rz0 cache --dry-run` live adapter plus strict fixture path and one exact runtime-cache-file plan/apply lane; no recursive cleanup |
| `modules/security-integrity/` | Classifies exact digest match/mismatch observations | Core `rz0 integrity --dry-run --fixture` and bounded exact-file path; report-only and no trusted baseline |

The foundation owns producer/category binding, privacy, protected-data policy,
resource ceilings, evidence identity, sorting, summary counts, and authority
refusal. Modules must not duplicate or loosen those controls.

## Updater exception

The updater package now contains more than a synthetic classifier:

- strict finding input and deterministic finding reports;
- exact-input-bound action plans and serial queue review;
- Homebrew JSON plus APT, DNF, Pacman, MacPorts, and Flatpak JSON parser slices;
- probe specifications for Homebrew, MacPorts, Winget, APT, DNF, Pacman,
  Zypper, Snap, and Flatpak;
- a separate stdin/stdout development binary;
- core integration for fixture, captured-output, and explicit live probes.

Winget, Zypper, and Snap parsers currently fail closed as not yet locale-safe.
The Flatpak parser requires the forced `C` locale, strict JSON columns, and a
12-character remote commit for each exact app/architecture/branch ref. Manager
probes and action plans do not independently authorize a write. The core's explicitly confirmed manager-update lane remains a narrow
pre-alpha exception with Linux native-ELF identity-bound spawn and canonical
external-effect receipts, but open macOS/Windows identity, OS isolation, full
cancellation, rollback, manager-specific recovery, and platform-proof gates. See
[`action-planning.md`](action-planning.md) and
[`project-status-and-resumption.md`](project-status-and-resumption.md).

## Other classifier limits

Uninstall now receives one selected live catalog record from core, can build a
sealed manager action plan, and has one narrow foundation-owned manager apply
boundary. It still has no dependent-package review, quarantine, rollback, or
broad cleanup lane. Leftovers now has bounded runtime-owned read-only discovery
plus an explicit exact-module-file dry-run plan and confirmation-bound exact
quarantine lane, while security/integrity has a bounded exact-file read adapter
and cache has bounded known-root read-only discovery. The core `restore` command
is deliberately outside discovery: it accepts only one validated runtime.zero
quarantine record and restores only its exact original cache/module path after
fresh confirmation.

Across these non-updater families there is:

- no independent complete live Windows/macOS/Linux adapter;
- no signed lifecycle artifact or installation/activation path;
- no direct TUI mutation flow;
- no broad package-manager mutation, recursive cleanup, or network action.

Their manifests remain `planned`. Core does not install, activate, or execute
third-party/module lifecycle code. The updater and the narrow manager-native
uninstall boundary are foundation-owned exceptions with shared transaction and
receipt paths.

Current tests establish only that:

- installed and manager-owned evidence is required for uninstall candidates and
  exact executable identity is required for a planned manager action;
- only exact runtime-owned leftover/cache evidence can become a quarantine
  candidate;
- protected and unknown evidence stays blocked;
- an integrity mismatch is high-risk report evidence, never remediation
  authority.

## Completion gate

Before any remaining family advances, add:

1. complete requirements, non-goals, privacy classes, roots/managers, and
   supported-platform tables;
2. bounded valid/missing/duplicate/malformed/oversized/locale/permission/
   symlink/reparse/partial-failure fixtures;
3. exact input provenance and explicit read capabilities;
4. live adapters that preserve useful unavailable/partial states;
5. shared finding-bound dry-run plans where mutation applies;
6. exact confirmation, transaction, rollback/quarantine, cancellation,
   isolation, and post-action verification through foundation APIs;
7. CLI/JSON/TUI, accessibility, performance, privacy, security, support, and
   final-artifact proof;
8. every required Windows/macOS/Linux release-ledger cell.

See [`completion-checklist.md`](completion-checklist.md) for the full 1.0 list.
