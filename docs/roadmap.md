# Roadmap

The roadmap is ordered by safety and contract dependency rather than by visual
surface. Foundation/TUI work advanced ahead of feature modules; that does not
approve skipping the inventory and trust gates below.

## Phase 1 — bootstrap (complete foundation baseline)

- Public repository and Rust CLI command `rz0`.
- Centralized brand metadata and public safety/security/contribution docs.
- Minimal static site at `https://rz0.neuman.dev`.
- Read-only `doctor`, `modules`, and dry-run `scan` surfaces.
- Read-only Ratatui/Crossterm dashboard with scriptable text/JSON fallbacks.
- Manifest validation, local SHA-256 package integrity, install dry-run planning,
  store plan/status, registry/receipt validation, and explicit store scaffolding.

## Phase 2 — inventory contracts and primitives (in progress)

- [x] Versioned, deterministic inventory JSON shape.
- [x] Explicit read-only/no-write/privacy fields before platform probes.
- [ ] Fixture-backed Windows process-PATH parsing and normalization.
- [ ] Valid, duplicate, missing, malformed, and unsupported-platform fixtures.
- [ ] Narrow read-only live process-PATH adapter after fixture proof.
- [ ] Persisted User/Machine PATH reads behind a separate review gate.
- [ ] Known executable discovery/version probes with bounded execution.
- [ ] Structured source status, duration, warnings, and logs.
- [ ] Optional package-manager and Windows app evidence after command/network and
  privacy behavior are separately reviewed.

See [`inventory-schema.md`](inventory-schema.md).

## Phase 3 — first-party Windows inventory module

- Separate, explicitly chosen first-party package.
- Read-only PATH, tool, package-manager, and application evidence.
- Deterministic text/JSON output and read-only TUI summary.
- No core-bundled feature behavior and no runtime module execution until its
  execution/trust boundary is separately approved.

## Phase 4 — updater modules

- Installed-only update planning.
- No surprise installs.
- Tool registry and denylist.
- CLI/dev/AI tool profile inspired by the original `aiup` need.

## Phase 5 — uninstall and leftovers

- Manager-native uninstall plans.
- Deep leftover scan in report-only mode.
- Risk-category review.
- Quarantine manifest and restore design before mutation.

## Phase 6 — interactive UX and site (foundation TUI implemented; parity queued)

- [x] Terminal review flow, focus regions, preview-only command rail, and
  responsive layout tiers.
- [ ] Manual cross-terminal/cross-platform UX verification.
- [ ] Website parity pass after separate visual approval.
- [ ] Optional framework migration or deployment automation only after explicit
  approval.

## Phase 7 — macOS/Linux adapters and distribution gates

- Homebrew/XDG/systemd/LaunchAgent adapters.
- Cross-platform compatibility verification.
- Dependency/security/license audit.
- Signing, release, package publishing, bootstrap, and automation only after
  separate threat modeling and explicit approval.
