# Domain Finding Classifier Modules

Five first-party packages consume the shared
`crates/finding-contract/`, but their maturity is no longer identical.

## Current package boundaries

| Package | Current source behavior | Live/core status |
| --- | --- | --- |
| `modules/updater/` | Classifies installed/manager-owned update evidence, parses selected manager output, builds finding-bound action plans and serial queues | Fixture, captured-output, and explicit live-probe paths exist; core owns the separate explicit apply lane |
| `modules/uninstall/` | Classifies synthetic or live installed-software evidence and builds optional finding-bound dry-run manager plans | Core `uninstall plan` uses this shared producer; no uninstall execution |
| `modules/leftovers/` | Classifies bounded runtime.zero-owned module/log evidence for conservative post-uninstall review | Core `rz0 leftovers --dry-run` live adapter plus strict fixture path; no quarantine |
| `modules/cache/` | Classifies bounded known-root cache evidence while preserving conservative ownership policy | Core `rz0 cache --dry-run` live adapter plus strict fixture path; no cleanup |
| `modules/security-integrity/` | Classifies exact digest match/mismatch observations | Core `rz0 integrity --dry-run --fixture` strict fixture path; report-only and no trusted baseline |

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
pre-alpha exception with Linux native-ELF identity-bound spawn and canonical
external-effect receipts, but open macOS/Windows identity, OS isolation, full
cancellation, rollback/recovery completion, and platform-proof gates. See
[`action-planning.md`](action-planning.md) and
[`project-status-and-resumption.md`](project-status-and-resumption.md).

## Other classifier limits

Uninstall now receives one selected live catalog record from core and can build
a sealed manager action plan, but it still has no process, elevation, dependent-
package review, quarantine, rollback, or execution lane. Leftovers now has
bounded runtime-owned read-only discovery, while security/integrity remains
synthetic-only and cache has bounded known-root read-only discovery. Across
these four families there is:

- no independent complete live Windows/macOS/Linux adapter;
- no authorized binary/process protocol or host write permission;
- no package-manager/filesystem mutation or network action;
- no execution-capable action pipeline;
- no signed lifecycle artifact or installation/activation path;
- no direct TUI mutation flow.

Their manifests remain `planned`. Core does not install or execute these
non-updater families. The updater is the separate foundation-owned exception
with the shared CLI/TUI manager apply lane.

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
