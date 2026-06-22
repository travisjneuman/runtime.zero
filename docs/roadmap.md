# Roadmap

## Phase 1 — bootstrap

- Public repository.
- Rust CLI command `rz0`.
- Centralized brand metadata.
- Safety, security, contribution, architecture, module, and roadmap docs.
- Minimal static site source and first live Cloudflare Worker deployment at `https://rz0.neuman.dev`.
- Read-only `doctor`, `modules`, and dry-run `scan` stubs.

## Phase 2 — foundation contracts and inventory primitives

- Module manifest model and registry contract.
- Local read-only module manifest validation.
- Core-vs-installed-module CLI output.
- JSON output contract for module registry.
- Windows read-only discovery for PATH, persisted PATH, package managers, app registry entries, and direct known tool paths.
- Structured logs.
- Test fixtures.

## Phase 3 — first-party inventory module

- Separate first-party Windows inventory module.
- Read-only PATH, tool, package-manager, and app evidence.
- Deterministic JSON output.

## Phase 4 — updater modules

- Installed-only update planning.
- No surprise installs.
- Tool registry and denylist.
- CLI/dev/AI tool profile inspired by the original `aiup` need.

## Phase 5 — uninstall and leftovers

- Manager-native uninstall plans.
- Deep leftover scan in report-only mode.
- Risk-category review.
- Quarantine manifest design.

## Phase 6 — interactive UX and site

- Terminal review flow.
- Static docs/site polish and stronger terminal-noir visual direction.
- CLI/site aesthetic alignment after the foundation command surface is ready for manual review.
- Optional framework migration or deployment automation only after explicit approval.

## Phase 7 — macOS/Linux adapters

- Homebrew/XDG/systemd/LaunchAgent adapters.
- Cross-platform release builds after automation is approved.
