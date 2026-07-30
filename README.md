# runtime.zero

**System Management Toolkit**  
Command: `rz0`

`runtime.zero` is a Rust-first, terminal-native foundation for safe system management. The core stays intentionally small: it owns the CLI, policy, output contracts, and module registry primitives while substantial capabilities ship as explicit modules instead of being bundled by default.

> Status: pre-alpha foundation plus separately built, read-only first-party inventory and report/export source packages. This repository is public early so the design and safety model are visible from the start. The core does not install or execute modules, and destructive modules are intentionally absent.

## The promise

`runtime.zero` is designed to feel like a dark terminal control surface while behaving like a careful system steward:

- report first;
- dry-run first;
- quarantine before delete;
- manager-native uninstall before file cleanup;
- no surprise installs;
- no credential/session cleanup without explicit approval;
- no persistence, malware behavior, evasion, or account actions.

## Supported command surface today

```bash
rz0 --version
rz0 --tui
rz0 --no-tui
rz0 --color auto|always|never
rz0 doctor
rz0 doctor --format json
rz0 modules
rz0 modules --format json
rz0 modules validate <manifest.json>
rz0 modules --from <directory> --format json
rz0 modules install --dry-run <package-dir-or-manifest>
rz0 store plan
rz0 store plan --format json
rz0 store status
rz0 store status --format json
rz0 store status --store-root tests/fixtures/store-roots/valid-registry-valid-receipt --format json
rz0 store init --dry-run
rz0 store init --yes
rz0 scan --dry-run
rz0 scan --dry-run --format json
```

Bare `rz0` opens the read-only TUI dashboard shell in an interactive terminal.
It uses raw key handling, so `q` exits without echoing typed input, and it
filters terminal key events so Windows key-release events do not double-advance
selection. The current interactive dashboard uses a Ratatui widget layer for bounded
componentized panels, status badges, numbered dossier sections, explicit focus regions, a navigation rail,
selected-section details, read-only command previews, Home/End jumps,
Tab/Shift+Tab focus cycling, arrow movement, and `j`/`k` keyboard shortcuts for
operator-style terminal use. It now chooses explicit wide, standard, compact,
and very-small layout tiers so constrained terminals keep visible focus and
read-only/preview-only labels instead of clipping into misleading panes. Esc
closes help/previews or backs out before quitting from the base navigation
focus. Use `rz0 --no-tui` for the scriptable text
dashboard, or `rz0 --json` for a machine-readable foundation dashboard.
`rz0 <subcommand>` remains scriptable and never opens the TUI.
`rz0 --tui` explicitly requests the full-screen TUI and fails clearly if the
terminal is non-interactive or automation is detected; plain `rz0` falls back
to the safe text dashboard in those contexts.

Color is explicit and accessible: `--color=auto` is the default,
`--color=never` disables ANSI even in the interactive TUI, and
`--color=always` forces color for supported human-readable surfaces. JSON
output never includes ANSI. The root dashboard JSON includes additive contract
metadata (`schema_version`, `contract`, `read_only`, and `writes_attempted`) so
automation can distinguish foundation review output from future mutating
module surfaces.

Current commands are read-only, dry-run, or explicit user-local store
scaffolding. They exist to prove the binary, brand metadata, test harness,
documentation foundation, TUI shell, module contract surface, and the first
versioned inventory output contract. Core `scan --dry-run --format json` emits
an intentionally empty schema-1 report. The separate `modules/inventory/`
workspace package now supplies fixture-backed and live read-only collectors; it
is not installed, loaded, or executed by the core.

## Core vs modules

The installed `rz0` foundation is not meant to contain every feature. It should remain useful with zero optional modules installed:

- `core.cli` handles command routing and output.
- `core.policy` defines shared safety metadata and future mutation gates.
- `core.registry` lists core primitives and explicitly installed modules.

First-party feature modules are planned as separate install/use choices. A full bundle may exist later as a convenience distribution, but it should not redefine the core. Third-party modules require a hardened trust model before support is added.

The foundation can validate local module manifests without executing module
code. Installed manifests must also pass local SHA-256 integrity checks for
explicitly listed package files:

```bash
rz0 modules validate path/to/rz0-module.json
rz0 modules --from path/to/installed-modules --format json
rz0 modules install --dry-run path/to/module-package
```

This is local, read-only validation and planning only. The install planner
reports proposed locations and state changes, but it does not write files,
install, update, fetch, trust, enable, or run modules.

The dry-run planner also reports future local store and CLI/TUI routing
contract metadata in JSON output. These fields describe where future state would
live and why explicit subcommands remain scriptable; they do not create files.

The first feature-module source package lives at
[`modules/inventory/`](modules/inventory/). It reads process PATH on supported
platforms, reads persisted User/Machine PATH on Windows, detects a bounded set
of known executables, supports opt-in Unix version probes with cleared
environment, shared bounded drains/deadlines/process-group teardown, and can read
normalized platform application evidence when explicitly requested. Windows
version probes fail closed pending race-free production containment.
Windows uses read-only uninstall registry views, macOS enumerates only direct
`.app` bundles under known roots, and Linux parses bounded XDG desktop entries.
Paths are redacted by default; raw local values require the explicit
`--include-raw-paths` flag. It does not run package
managers, modify the system, or make the module installable through `rz0`.

The second source package, [`modules/report-export/`](modules/report-export/),
accepts a bounded strict inventory/diagnostics envelope on standard input and
emits only a deterministic summary to standard output. The shared
`crates/support-contract/` owns input validation, domain-separated digests,
privacy omissions, bounds, and non-authority fields. Raw reports, paths,
identities, application names, process output, and free-form warnings are not
embedded. The module has no path/network options and is not executed by core.

A separate `crates/module-trust/` contract now verifies local detached Ed25519
signatures with public test keys only. It does not provide signing, production
keys, installation, activation, or module execution. Schema-1 staging plans and
integration-test-only OS-temp helpers now exercise atomic staging and
quarantine/restore failure semantics without adding a production filesystem
mover. A separate schema-1 process protocol keeps module execution unauthorized.
An explicit-feature integration lane executes only a Cargo-built test helper to
exercise bounded JSON framing, environment clearing, output draining, timeout
kill/reap, and fail-closed errors; it is not linked to the core or inventory
module. A separate schema-1 production execution assessment enumerates all
artifact, capability, executable-identity, process, runtime, and transaction
gates while remaining structurally unable to authorize execution. The
`crates/artifact-identity/` hashes and identifies one receipt-relative file
through the same returned handle and can issue a non-authorizing Linux/Windows
spawn-identity lease; macOS deliberately remains unsupported pending an exact
primitive. `crates/capability-contract/` supplies one shared least-privilege
vocabulary plus exact partition/list validation for manifests, protocols, and
action plans while granting no authority.
`crates/cancellation-contract/` provides a one-atomic first-reason cancellation
and monotonic-deadline primitive consumed by guarded timeout polling and every
synchronized commit-coordinator boundary. Partial commit cancellation requires
recovery and never auto-rolls back. `crates/module-lifecycle/` owns dry-run,
digest-bound install/activate/invoke/repair/migrate/upgrade/deactivate/uninstall
transitions and their exact foundation gates. `crates/privacy-contract/` owns
bounded report-local redaction, `crates/configuration-contract/` owns immutable
fail-closed defaults, `crates/diagnostics-contract/` binds that policy into
privacy-safe `doctor` output, `crates/support-contract/` owns privacy-reviewed
summary exports from validated reports, `crates/performance-contract/` owns bounded final-
artifact budgets/evidence, and `crates/process-host/` owns bounded process I/O plus
fail-closed handle/descriptor and test-containment primitives.
`crates/finding-contract/` owns path-free typed ownership/data-class/confidence/
risk/disposition evidence shared by updater, uninstall, leftovers, cache, and
integrity modules; findings cannot authorize actions. `crates/confirmation-contract/`
binds validated plan/dry-run/write-set/state
hashes to a five-minute interactive phrase and single-use consumption record
while remaining unable to authorize execution. `crates/transaction-contract/`
supplies a bounded hash-chained state machine, exclusive immutable snapshots,
single-use confirmation publication, committed-head receipt binding, registry-
last atomic coordination, and conservative recovery decisions without automatic
mutation. `crates/error-contract/` supplies stable machine error
codes with fail-closed privacy and retry semantics. `crates/resource-contract/`
centralizes shared byte/record/timeout/process ceilings so modules cannot
silently expand them. `crates/validation-contract/` owns allocation-free bounded
ID/version/hash/path grammar so parsers cannot silently diverge.
`crates/secure-fs/` owns held-directory-relative state operations, locks, privacy
checks, and atomic publication; Windows NT operations and strict owner/DACL
inspection are compile-checked, while Windows store mutation remains blocked
pending safe initial ACL creation and runtime proof. `crates/registry-contract/` owns the
canonical installed-state shape and digest. `crates/release-contract/` generates the exact bounded target × seven-module ×
12-stage evidence ledger while remaining unable to authorize release. See
[`docs/artifact-identity.md`](docs/artifact-identity.md),
[`docs/capability-contract.md`](docs/capability-contract.md),
[`docs/cancellation-contract.md`](docs/cancellation-contract.md),
[`docs/privacy-contract.md`](docs/privacy-contract.md),
[`docs/configuration-contract.md`](docs/configuration-contract.md),
[`docs/diagnostics-contract.md`](docs/diagnostics-contract.md),
[`docs/support-report-contract.md`](docs/support-report-contract.md),
[`docs/process-host-foundation.md`](docs/process-host-foundation.md),
[`docs/performance-contract.md`](docs/performance-contract.md),
[`docs/module-lifecycle-contract.md`](docs/module-lifecycle-contract.md),
[`docs/finding-contract.md`](docs/finding-contract.md),
[`docs/confirmation-contract.md`](docs/confirmation-contract.md),
[`docs/error-contract.md`](docs/error-contract.md),
[`docs/resource-contract.md`](docs/resource-contract.md),
[`docs/validation-contract.md`](docs/validation-contract.md),
[`docs/secure-filesystem.md`](docs/secure-filesystem.md),
[`docs/installed-registry-contract.md`](docs/installed-registry-contract.md),
[`docs/release-acceptance.md`](docs/release-acceptance.md),
[`docs/signature-verification.md`](docs/signature-verification.md),
[`docs/transaction-simulation.md`](docs/transaction-simulation.md),
[`docs/transaction-journal.md`](docs/transaction-journal.md),
[`docs/module-process-protocol.md`](docs/module-process-protocol.md), and the
equal-platform/equal-module completion contract in
[`docs/production-readiness.md`](docs/production-readiness.md), rolling
[`docs/support-policy.md`](docs/support-policy.md), explicit
[`docs/windows-compatibility.md`](docs/windows-compatibility.md), and no-paid-signing
[`docs/free-release-distribution.md`](docs/free-release-distribution.md).

The same future store/routing contract can be inspected independently of module
install planning:

```bash
rz0 store plan
rz0 store plan --format json
rz0 store status
rz0 store status --format json
rz0 store status --store-root tests/fixtures/store-roots/valid-registry-valid-receipt --format json
rz0 store init --dry-run
rz0 store init --yes
```

`store plan`, `store status`, and `store init --dry-run` are read-only.
`store init --yes` is separately explicit and write-capable. `store plan` reports
the platform-specific
user-local store roots, registry and transaction paths, example
receipt/quarantine/rollback paths, forbidden path classes, and current CLI/TUI
launch-routing interpretation. `store status` checks whether those future paths
already exist and also parses an existing `installed-modules.json` registry if
present. It reports absent, empty, valid, invalid, or unreadable registry state,
schema version, installed module count, duplicate IDs, malformed records, and
unsafe path references. When a valid registry record references an existing
receipt, `store status` validates that receipt shape and cross-checks module
ID/version and store-relative paths. It still does not create directories,
write state, repair anything, trust modules, execute code, or imply modules are
active.

`store status --store-root <path>` is a read-only fixture/support override for
inspecting a supplied local store root instead of the real user-local store. It
reports missing roots as absent and wrong filesystem types as invalid; it never
initializes, repairs, migrates, or writes the supplied path.

`store init --dry-run` reports the exact user-local store scaffolding that a
future-ready local store needs. On Unix, `store init --yes` creates runtime.zero-
owned user-local directories, an empty schema-1 registry, and an initialization
marker through held-parent no-follow operations. It is idempotent and refuses to
repair or overwrite invalid existing state. Windows apply currently fails closed
until reviewed owner/DACL policy and NT runtime evidence exist. It does not
install modules, copy packages, execute code, fetch remote content, edit PATH,
or create services, tasks, registry entries, persistence, releases, or
bootstrap hooks.

## Platform target

Windows, macOS, and Linux are equal release priorities. The explicit matrix
includes Windows 11/10/8.1/8/7 and Server 2008–2025 real variants, macOS Tahoe 26
through Ventura 13 as the initial backward set, and rolling Ubuntu, Debian, RHEL,
and Arch generations across architectures that actually existed. Vendor-retired
systems are compatibility investigations, not current security-support claims.
No platform cell becomes supported without final-artifact runtime evidence. See
[`docs/support-policy.md`](docs/support-policy.md) and
[`docs/windows-compatibility.md`](docs/windows-compatibility.md).

## Development

```bash
cargo test --workspace
cargo run --
cargo run -- --no-tui
cargo run -- --json
cargo run -- --version
cargo run -- doctor
cargo run -- doctor --format json
cargo run -- modules
cargo run -- modules --format json
cargo run -- modules validate path/to/rz0-module.json
cargo run -- modules install --dry-run path/to/module-package
cargo run -- store plan
cargo run -- store plan --format json
cargo run -- store status
cargo run -- store status --format json
cargo run -- store status --store-root tests/fixtures/store-roots/valid-registry-valid-receipt --format json
cargo run -- store init --dry-run
cargo run -- store init --dry-run --format json
cargo run -- scan --dry-run
cargo run -- scan --dry-run --format json
cargo run -p rz0-module-inventory -- --fixture modules/inventory/tests/fixtures/valid.json --format json
cargo run -p rz0-module-inventory -- --format json
cargo run -p rz0-module-inventory -- --include-apps --format json
cargo run -p rz0-module-report-export -- --format json < report-export-input.json
python3 -m unittest scripts.tests.test_prepare_macos_dmg
scripts/build-package.sh aarch64-apple-darwin /tmp/runtime-zero-package
scripts/build-dmg.sh aarch64-apple-darwin /tmp/runtime-zero-dmg
scripts/benchmark_final_artifact.py --binary /path/to/rz0 --target aarch64-apple-darwin --source-commit <commit> --output /tmp/rz0-performance.json
scripts/smoke_terminal_artifact.py --binary /path/to/rz0 --target aarch64-apple-darwin --source-commit <commit> --output /tmp/rz0-terminal.json
cargo deny check
```

`cargo deny check` is an optional manual dependency-policy check and requires a
separately installed `cargo-deny`; the project does not auto-install it.

## Local install for development

To make `rz0` available from a normal PowerShell terminal on a development
machine, use the local-only install script:

```powershell
.\scripts\install-local.ps1 -DryRun -AddToPath
.\scripts\install-local.ps1 -AddToPath
```

The script builds the checked-out binary, copies it to
`%USERPROFILE%\.local\bin\rz0.exe`, writes a local install marker, and adds that
directory to the **user** PATH only when `-AddToPath` is supplied. Open a new
PowerShell terminal after installing before expecting `rz0` to resolve outside
the repository.

Rollback is also local and explicit:

```powershell
.\scripts\uninstall-local.ps1 -DryRun -RemovePath
.\scripts\uninstall-local.ps1 -RemovePath
```

See [`docs/local-install.md`](docs/local-install.md) for the safety boundaries
and options, including how rollback treats pre-existing user PATH entries. This
is not a public release, installer, package manager, bootstrap command, or
install-from-internet flow.

## Architecture

The project is intentionally modular:

- Rust CLI core for command parsing, action planning, policy, logs, JSON output, and quarantine/restore.
- Platform adapters for Windows, macOS, and Linux.
- Optional modules for update, uninstall, leftover scan, cleaner, security/integrity checks, and future ideas.
- Read-only foundation TUI shell for local review, using crossterm for raw
  terminal lifecycle and Ratatui for the interactive widget dashboard, with
  componentized panels/status badges, focus regions, navigation rail, numbered dossier sections, selected-section
  panel, foundation status cards, read-only command previews, Home/End and
  `j`/`k` navigation, and command rail; subcommands remain the stable
  automation/script surface.

See [`docs/architecture.md`](docs/architecture.md),
[`docs/module-system.md`](docs/module-system.md),
[`docs/manifest-validation.md`](docs/manifest-validation.md), and
[`docs/foundation-readiness.md`](docs/foundation-readiness.md). See
[`docs/store-and-routing-contract.md`](docs/store-and-routing-contract.md) for
the local module store, store initialization, and CLI/TUI launch-routing
contract.

[`docs/tui.md`](docs/tui.md) for the read-only terminal UI foundation,
keyboard behavior, rendering boundaries, and brand/theme structure. See
[`docs/inventory-schema.md`](docs/inventory-schema.md) for the inventory report
and collector contract. Module execution/trust prerequisites are in
[`docs/module-trust-and-execution.md`](docs/module-trust-and-execution.md), with
the bounded test-key contract in
[`docs/signature-verification.md`](docs/signature-verification.md). Test-only
staging/quarantine/restore behavior is documented in
[`docs/transaction-simulation.md`](docs/transaction-simulation.md), and the
no-execution module contract and explicit-feature test-helper transport in
[`docs/module-process-protocol.md`](docs/module-process-protocol.md). Future
update/uninstall/quarantine boundaries are in
[`docs/action-planning.md`](docs/action-planning.md). The current manual
dependency/license/validation snapshot is in
[`docs/dependency-and-validation-audit.md`](docs/dependency-and-validation-audit.md).
The finite production definition, foundation/module ownership boundary, and
Windows/macOS/Linux module acceptance matrix are in
[`docs/production-readiness.md`](docs/production-readiness.md).
Website TUI
parity is tracked in [`docs/website-tui-parity-backlog.md`](docs/website-tui-parity-backlog.md)
so the static site can later follow the real terminal TUI without drifting.

## Brand system

The canonical public brand guide is [`BRAND.md`](BRAND.md).

Current direction: **Dossier Navy / Burnished Brass** — blackened navy,
graphite panels, bone-white type, burnished-brass operational accents, muted
blue-gray metadata, and red only for danger/error/destructive states.

Owner-provided candidate assets live under [`assets/brand/`](assets/brand/).
They are candidates, not final locked identity assets.

## Repository hygiene

The project root is intentionally kept small and conventional. Foundation source
belongs in `src/`, shared contract libraries in `crates/`, separately built
feature modules in `modules/`, product docs in `docs/`, site material in `site/`,
brand assets in `assets/brand/`, and tests/fixtures beside the narrowest owning
package. Durable planning and session artifacts belong
in `_meta.notes`, not as loose root files.

## Website

The first static landing page is live at [`https://rz0.neuman.dev`](https://rz0.neuman.dev) and its source lives in [`site/`](site/). It is deployed through the connected Cloudflare Worker project `runtime-zero` using `site/` as the static output directory.

This first version is dependency-free and public-safe, but the visual direction is still provisional. Website visual editing is currently paused until stronger reference examples are reviewed. Future site work should align to [`BRAND.md`](BRAND.md), avoid red as a brand accent, keep claims honest, avoid unsafe direct-run commands, and preserve the static deployment unless a framework migration is separately approved.

## License

Apache-2.0. See [`LICENSE`](LICENSE).
