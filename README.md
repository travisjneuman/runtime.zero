# runtime.zero

**System Management Toolkit**  
Command: `rz0`

`runtime.zero` is a Rust-first, terminal-native foundation for safe system management. The core owns shared policy, contracts, bounded inventory, and explicit mutation lanes; domain writes still require exact plans, confirmation, transactions, and post-action verification.

> **Current pre-alpha snapshot (reviewed 2026-08-20):** the installed surface provides bounded software/package/service inventory, source-identity grouping, a task-first five-workspace TUI, native monitoring, privacy-reviewed local support summaries, and a provider-driven CLI manager-update coordinator. The TUI now starts with a loading shell and groups Rust/AI/developer tools in a Toolchain workspace; the old command rail and six-section chrome are retired. The coordinator resolves system managers, language/package environments, known self-updaters, multiple npm prefixes, and declared application update metadata without guessing ownership; `--apply` executes native manager commands with plan-bound executable identity, isolated manager environments, optional non-interactive sudo elevation, durable external-effect receipts, self-updater replacement handling, and fresh post-action verification. It remains pre-alpha: Windows production containment, private/UI-only app channels, exact recovery completion, native rollback, and final-artifact runtime matrices remain incomplete. Start with the [`user guide`](docs/user-guide.md), [`TUI guide`](docs/tui.md), [`current status`](docs/project-status-and-resumption.md), [`engineering handoff`](docs/engineering-handoff.md), and [`documentation guide`](docs/documentation-index.md).

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
rz0 modules lifecycle-plan <operation> --dry-run --module-id <id> --from-state <state> --to-state <state> [--from-version <version>] [--to-version <version>]
rz0 store plan
rz0 store plan --format json
rz0 store status
rz0 store status --format json
rz0 store status --store-root tests/fixtures/store-roots/valid-registry-valid-receipt --format json
rz0 store init --dry-run
rz0 store init --yes
rz0 apps
rz0 apps --format json
rz0 report
rz0 report --format json
rz0 uninstall plan <installed-software-id>
rz0 uninstall plan <installed-software-id> --executable /opt/homebrew/bin/brew --format json
rz0 completions bash|zsh|fish|powershell
rz0 scan --dry-run
rz0 scan --dry-run --format json
rz0 monitor --format text
rz0 monitor --format json
rz0 toolchain
rz0 toolchain --format json
rz0 updates --dry-run --fixture tests/fixtures/updater/evidence.json --plan --queue --format json
rz0 updates --dry-run --manager homebrew-formula --manager-output /tmp/out.json --executable /opt/homebrew/bin/brew --plan --queue --format json
rz0 updates --dry-run --probe --manager homebrew-formula --executable /opt/homebrew/bin/brew --allow-network-read --plan --queue --format json
rz0 updates --dry-run --all-providers --allow-network-read --plan --queue --format json
rz0 updates --recovery-status --transaction <exact-transaction-id>
rz0 updates --apply --probe --manager homebrew-formula --executable /opt/homebrew/bin/brew --allow-network-read --allow-network-write --action <exact-action-id> --accept-no-rollback --challenge-issued-unix-seconds <issued> --confirm '<exact-phrase>'
rz0 updates --apply --all-providers --allow-network-read --allow-network-write --accept-no-rollback
rz0 updates --apply --all-providers --allow-network-read --allow-network-write --action <exact-action-id> --accept-no-rollback --challenge-issued-unix-seconds <issued> --confirm '<exact-phrase>'
```

Bare `rz0` opens the task-first five-workspace dashboard in an interactive
terminal. It renders a loading shell before the full local snapshot, then uses
raw key handling, mouse capture, visible selection, a separate selected-context
panel, and direct review entry points. Enter opens selected-item details; the
mouse wheel advances the list by a bounded increment; `m` selects the System
workspace, which uses native macOS/Linux/Windows
collectors and does not require a separate btop/top/task-manager install.
`u` scans provider availability; visible Review action (`U` compatibility
shortcut) targets the highlighted provider-backed
update candidate, presents the exact manager command and confirmation phrase,
and enters the shared confirmation-bound update lane. `r` refreshes the local
snapshot. The dashboard does not silently execute destructive actions, and it
does not present unavailable module or uninstall operations as implemented.
The current Ratatui widget layer provides two-panel workspaces, semantic
labels, section navigation, a live native system monitor, Home/End jumps,
Tab/Shift+Tab focus cycling, arrow and
`j`/`k` movement, `/` search, `f` filter cycling, `s` sort cycling, and
wide/standard/compact layout tiers that keep the selected row visible. Esc
closes details/help or backs out before quitting. Use `rz0 --no-tui` for the
scriptable text dashboard, or `rz0 --json` for a machine-readable foundation
dashboard.
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

Inventory, diagnostics, and evidence collection remain read-only by design;
that is different from the platform being unable to act. The installed core
embeds the bounded first-party inventory adapter: `rz0 apps` lists path-free
local software, `rz0 scan --dry-run` collects live redacted evidence, and the
TUI shows installed applications/packages, source identifiers, service and
persistence counts, versions when available, and ownership-specific uninstall
review commands. Provider-native updates use `rz0 updates --apply` for the
discovered system managers, language/package environments, self-updaters, and
declared application channels; protected system applications and uninstall
reviews remain blocked from execution until their own transaction lanes are
complete.

## Core vs modules

The installed `rz0` foundation is not meant to contain every domain feature.
It remains useful with zero optional modules installed because inventory is a
built-in foundation adapter, while executable actions are owned by explicit
foundation lanes. The end goal is a full system-management platform: every
feature family or provider becomes an independently versioned module that an
end user can install, enable, configure, disable, update, repair, or uninstall
for their use case. The initial seven families are the first release gate, not
the ceiling. Read [`docs/engineering-handoff.md`](docs/engineering-handoff.md)
for the complete product horizon and shift plan.

- `core.cli` handles command routing and output.
- `core.policy` defines shared safety metadata and executable action gates.
- `core.registry` lists core primitives and explicitly installed modules.

**Implementation standard:** a feature is `implemented` only when its normal
user path is callable, its result is observable, and its failure/recovery path
is tested. A schema, fixture parser, dry-run planner, or preview is a
foundation component, not a completed end-user capability. Safe confirmation,
rollback, and privilege gates may pause an action, but they must not hide an
otherwise available action behind permanent read-only wording.

First-party feature modules are planned as separate install/use choices. The
future module platform must distinguish installed, enabled, active,
degraded/blocked, and action-authorized states. Disabling a module must stop
its collection, network work, scheduling, UI actions, and mutation while
preserving its settings, evidence, and receipts; uninstall is a separate
explicit, data-retention-aware lifecycle. A full bundle may exist later as a
convenience distribution, but it should not redefine the core. Third-party and
remote modules require hardened trust, capability, isolation, revocation, and
support models before support is added.

The foundation can validate local module manifests without executing module
code. The fixture/captured-output `rz0 updates --dry-run` surface can classify
updater evidence and emit a serial review queue. The explicit `--probe` path
runs one bounded, cleared-environment manager query after requiring an
allowlisted absolute executable path and `--allow-network-read`; `--all-providers`
performs a provider-driven review of installed system managers,
language/package environments, known self-updaters, and declared application
update metadata. On macOS this includes Homebrew formulae/casks, MacPorts, Mac
App Store via `mas` when installed, Apple Software Update, npm prefixes,
crates.io Cargo installs, AIUP-managed native tools, Warp's standalone signed
CLI store, Electron/Squirrel GitHub metadata, and observed Sparkle channels;
other platforms use the providers native to that host. Missing, delegated,
observed-only, and unsupported sources remain explicit rather than being
treated as universal coverage. `--apply` is the
separate write lane and additionally requires `--allow-network-write`, exact
confirmation, an initialized private store, journal/receipt publication, and
fresh verification. Linux binds a direct native ELF manager's retained opened
identity to `/proc/self/fd` spawn; macOS uses last-moment path identity/digest
revalidation; Windows remains blocked pending production process-image
containment. The network flags are explicit intent rather than an OS network
sandbox. See the current-status guide
before evaluating this lane. Installed manifests must also pass local SHA-256 integrity checks
for explicitly listed package files:

```bash
rz0 modules validate path/to/rz0-module.json
rz0 modules --from path/to/installed-modules --format json
rz0 modules install --dry-run path/to/module-package
```

Module validation and installation planning remain local and bounded. The
current module planner does not fetch, trust, activate, or run module code;
module installation writes remain a separate lifecycle implementation and must
not be confused with manager update execution. Target commands such as
`rz0 modules enable`, `disable`, `configure`, `repair`, and `uninstall` are not
current commands until the foundation-owned lifecycle, registry publication,
receipts, recovery, and TUI path are implemented together.

The dry-run planner also reports future local store and CLI/TUI routing
contract metadata in JSON output. These fields describe where future state would
live and why explicit subcommands remain scriptable; they do not create files.

The first-party inventory source package lives at
[`modules/inventory/`](modules/inventory/), and its library is embedded by the
installed core as the local read-only collector. It reads process PATH on supported
platforms, reads persisted User/Machine PATH on Windows, detects a bounded set
of known executables, supports opt-in Unix version probes with cleared
environment, shared bounded drains/deadlines/process-group teardown, and can read
normalized platform application evidence when explicitly requested. Windows
version probes fail closed pending race-free production containment.
Windows uses read-only uninstall/service registry views; macOS enumerates direct
`.app` bundles, bounded Homebrew Cellar/Caskroom and MacPorts metadata, installer
receipts, and launchd labels; Linux parses bounded XDG desktop entries, direct
dpkg/pacman metadata, and systemd unit labels.
Paths are redacted by default; raw local values require the explicit
`--include-raw-paths` flag. It does not run package
managers or modify the system. Its separate development binary remains
available for fixture and adapter testing, while `rz0` owns the user-facing
catalog, scan, and TUI surfaces.

The report/export source package, [`modules/report-export/`](modules/report-export/),
accepts a bounded strict inventory/diagnostics envelope on standard input and
emits only a deterministic summary to standard output. The shared
`crates/support-contract/` owns input validation, domain-separated digests,
privacy omissions, bounds, and non-authority fields. Raw reports, paths,
host/user identities, application/service names, credentials, process output,
and free-form warnings are not embedded. The module has no path/network options
and is not executed by core; `rz0 report` calls the same shared builder directly
over redacted live evidence.

Updater, uninstall, leftovers, cache, and security/integrity have separate
source-level domain packages under `modules/`. Updater consumes captured or
explicit live manager evidence. Uninstall accepts selected live installed-
software evidence to produce a non-authorizing finding and optional sealed dry-
run manager action plan. Leftovers, cache, and integrity remain synthetic. They
do not provide uninstall/cleanup execution, elevation, signed lifecycle
activation, or production support. See
[`docs/domain-classifier-modules.md`](docs/domain-classifier-modules.md).

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
through the same returned handle and can issue a non-authorizing Linux lease or
macOS path-revalidation binding; production Windows spawn containment remains
incomplete. `crates/capability-contract/` supplies one shared least-privilege
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
inspection are compile-checked, and store initialization now uses that guarded
path while failing closed on unsuitable inherited ACLs. Public Windows store
support remains blocked pending safe initial ACL creation behavior and runtime
proof. `crates/registry-contract/` owns the
canonical installed-state shape and digest. `crates/release-contract/` generates the exact bounded target × seven-module ×
12-stage evidence ledger while remaining unable to authorize release. See
[`docs/artifact-identity.md`](docs/artifact-identity.md),
[`docs/capability-contract.md`](docs/capability-contract.md),
[`docs/cancellation-contract.md`](docs/cancellation-contract.md),
[`docs/privacy-contract.md`](docs/privacy-contract.md),
[`docs/configuration-contract.md`](docs/configuration-contract.md),
[`docs/diagnostics-contract.md`](docs/diagnostics-contract.md),
[`docs/support-report-contract.md`](docs/support-report-contract.md),
[`docs/domain-classifier-modules.md`](docs/domain-classifier-modules.md),
[`docs/process-host-foundation.md`](docs/process-host-foundation.md),
[`docs/system-monitor.md`](docs/system-monitor.md),
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
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --workspace --locked --all-features
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo run --locked --
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
cargo run -- report --format json
cargo run -- completions bash
cargo run -p rz0-module-inventory -- --fixture modules/inventory/tests/fixtures/valid.json --format json
cargo run -p rz0-module-inventory -- --format json
cargo run -p rz0-module-inventory -- --include-apps --format json
cargo run -p rz0-module-report-export -- --format json < report-export-input.json
python3 -m unittest scripts.tests.test_prepare_macos_dmg
scripts/build-package.sh aarch64-apple-darwin /tmp/runtime-zero-package
scripts/build-dmg.sh aarch64-apple-darwin /tmp/runtime-zero-dmg
scripts/benchmark_final_artifact.py --binary /path/to/rz0 --target aarch64-apple-darwin --source-commit <commit> --output /tmp/rz0-performance.json
scripts/smoke_terminal_artifact.py --binary /path/to/rz0 --target aarch64-apple-darwin --source-commit <commit> --output /tmp/rz0-terminal.json
python3 scripts/verify_release_package.py --archive target/release-package-<commit>/runtime-zero-0.1.0-aarch64-apple-darwin.zip --checksum-file target/release-package-<commit>/runtime-zero-0.1.0-aarch64-apple-darwin.zip.sha256 --source-commit <commit> --target aarch64-apple-darwin
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

- Rust CLI core for command parsing, built-in bounded inventory/monitoring, policy, contracts, JSON output, non-authorizing planning/recovery contracts, and the narrow explicit manager-update coordinator.
- Platform adapters for Windows, macOS, and Linux.
- Separately built first-party module families for inventory, updater domain logic, uninstall, leftovers, cache, security/integrity, and report/export. These are the initial release-gated families; the long-term catalog also includes developer/AI tooling, services/persistence, storage/data hygiene, security, network/hardware, OS settings, backup/recovery, automation, account/provider, and explicitly separated remote/fleet modules. Their executable lifecycle remains planned.
- Interactive local-software TUI using crossterm for raw/mouse terminal
  lifecycle and Ratatui for the widget dashboard, with componentized panels,
  visible selected rows, section navigation, details, mouse-wheel scrolling,
  a live native system monitor, Home/End and `j`/`k` navigation, and exact CLI
  action entry points;
  subcommands remain the stable automation/script surface.

Start with [`docs/engineering-handoff.md`](docs/engineering-handoff.md) for the
full-system-management end state, module contract, enable/disable semantics,
delivery waves, and next-shift checklist. Then use
[`docs/project-status-and-resumption.md`](docs/project-status-and-resumption.md)
for the current reviewed source snapshot, behavior, known limitations, evidence,
and dependency-ordered restart checklist. Use
[`docs/documentation-index.md`](docs/documentation-index.md) for document
precedence and the complete topic map. Then see
[`docs/architecture.md`](docs/architecture.md),
[`docs/module-system.md`](docs/module-system.md),
[`docs/manifest-validation.md`](docs/manifest-validation.md), and
[`docs/foundation-readiness.md`](docs/foundation-readiness.md). See
[`docs/store-and-routing-contract.md`](docs/store-and-routing-contract.md) for
the local module store, store initialization, and CLI/TUI launch-routing
contract.

See [`docs/tui.md`](docs/tui.md) for the terminal UI foundation, keyboard/mouse
behavior, rendering boundaries, and brand/theme structure. See
[`docs/inventory-schema.md`](docs/inventory-schema.md) for the inventory report
and collector contract. Module execution/trust prerequisites are in
[`docs/module-trust-and-execution.md`](docs/module-trust-and-execution.md), with
the bounded test-key contract in
[`docs/signature-verification.md`](docs/signature-verification.md). Test-only
staging/quarantine/restore behavior is documented in
[`docs/transaction-simulation.md`](docs/transaction-simulation.md), and the
no-execution module contract and explicit-feature test-helper transport in
[`docs/module-process-protocol.md`](docs/module-process-protocol.md). Update/uninstall/quarantine boundaries are in
[`docs/action-planning.md`](docs/action-planning.md). The current manual
dependency/license/validation snapshot is in
[`docs/dependency-and-validation-audit.md`](docs/dependency-and-validation-audit.md).
The finite production definition, foundation/module ownership boundary, and
Windows/macOS/Linux module acceptance matrix are in
[`docs/production-readiness.md`](docs/production-readiness.md). The consolidated
bullet-level remaining-work inventory is
[`docs/completion-checklist.md`](docs/completion-checklist.md).
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

This first version is dependency-free and public-safe, but its terminal mock predates the real task-first five-workspace TUI and current updater/monitor surfaces. Website visual or copy changes can deploy through the connected project and therefore remain a separate reviewed lane. Future site work should align to [`BRAND.md`](BRAND.md), mirror current product truth, avoid red as a brand accent, avoid unsafe direct-run commands, and preserve the static deployment unless a framework migration is separately approved.

## License

Apache-2.0. See [`LICENSE`](LICENSE).
