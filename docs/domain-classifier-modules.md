# Domain Finding Classifier Modules

Five first-party packages consume the shared
`crates/finding-contract/`, but their maturity is no longer identical.

## Current package boundaries

| Package | Current source behavior | Live/core status |
| --- | --- | --- |
| `modules/updater/` | Classifies installed/manager-owned update evidence, parses selected manager output, builds finding-bound action plans and serial queues | Fixture, captured-output, and explicit live-probe paths exist; core owns the separate explicit apply lane |
| `modules/uninstall/` | Classifies installed manager-record evidence | Synthetic input only; core has a separate non-executing Mac review UX |
| `modules/leftovers/` | Classifies exact runtime-owned orphan/executable evidence for possible quarantine | Synthetic input only |
| `modules/cache/` | Classifies exact runtime-owned cache evidence while preserving conservative ownership policy | Synthetic input only |
| `modules/security-integrity/` | Classifies exact digest match/mismatch observations | Synthetic input only and report-only |

The foundation owns producer/category binding, privacy, protected-data policy,
resource ceilings, evidence identity, sorting, summary counts, and authority
refusal. Modules must not duplicate or loosen those controls.

## Updater exception

The updater package now contains more than a synthetic classifier:

- strict finding input and deterministic finding reports;
- exact-input-bound action plans and serial queue review;
- Homebrew JSON plus APT, DNF, Pacman, and MacPorts parser slices;
- probe specifications for Homebrew, MacPorts, Winget, APT, DNF, Pacman,
  Zypper, Snap, and Flatpak;
- a separate stdin/stdout development binary;
- core integration for fixture, captured-output, and explicit live probes.

Winget, Zypper, Snap, and Flatpak parsers currently fail closed as not yet
locale-safe. Manager probes and action plans do not independently authorize a
write. The core's explicitly confirmed manager-update lane remains a narrow
pre-alpha exception with open executable-identity, isolation, cancellation,
rollback/recovery, and platform-proof gates. See
[`action-planning.md`](action-planning.md) and
[`project-status-and-resumption.md`](project-status-and-resumption.md).

## Other classifier limits

Uninstall, leftovers, cache, and security/integrity have:

- no live Windows/macOS/Linux discovery adapter;
- no binary/process protocol or host permission;
- no package-manager, filesystem, network, or platform API access;
- no action-plan generation or execution;
- no signed lifecycle artifact or installation/activation path;
- no TUI/core integration.

Their manifests remain `planned`. Core does not install or execute them.

Current synthetic tests establish only that:

- installed and manager-owned evidence is required for uninstall candidates;
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
