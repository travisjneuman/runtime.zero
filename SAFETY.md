# Safety Policy

`runtime.zero` is a system-management toolkit, so the safety bar is higher than a normal CLI.

## Defaults

- Report first.
- Dry-run first.
- Review/dashboard first for the TUI.
- Quarantine before delete.
- Prefer package-manager/native uninstall commands over direct file removal.
- Never surprise-install missing tools.
- Never automatically remove credentials, OAuth sessions, browser profiles, project workspaces, backups, or unknown user data.
- Require explicit confirmation before any mutating action.
- Keep substantial capabilities outside the core unless a user explicitly installs or enables the relevant module.
- Never execute remote module code or publish direct-run bootstrap commands before checksum/signing/release safety is designed.

## Module safety metadata

Capability names are foundation-owned through one shared contract; manifest,
protocol, and action schemas accept only their explicit subsets. Classification
does not grant OS authority.

Every module manifest must declare:

- capability class;
- risk level;
- whether it mutates the system;
- whether dry-run is required;
- whether explicit confirmation is required;
- whether quarantine/rollback is supported;
- whether remote execution is allowed.

Current core metadata sets remote execution to `false`. Optional modules are not bundled, installed, or executed by default.

## Manifest loading boundary

The foundation may read local JSON module manifests for validation and registry
listing. That loader is intentionally narrow:

- no remote URLs;
- no executable module code loading;
- no scripts or hooks;
- no install/update/uninstall side effects;
- install planning is dry-run only;
- no recursive drive scans;
- bounded manifest file size;
- bounded local package file integrity checks;
- third-party manifests rejected until the trust model exists.

Validation failure must be reported as data, not repaired automatically.

The separate module-trust crate verifies detached Ed25519 signatures against
caller-selected public test keys only. It binds package identity/version and an
exact manifest SHA-256, rejects revoked or unauthorized test keys, and has no
signer, private/production key, installer integration, activation, or execution path. Signature
success is not permission to trust or run a package.

Filesystem transaction simulations are integration-test-only. They require a
marked/prefixed direct child of the canonical OS temp root, normalized relative
paths, create-new destinations, digest/size verification, and symlink rejection.
Staging atomically publishes only complete fixture bytes. Quarantine verifies a
copy before removing the fixture original; injected failure keeps both copies;
restore refuses occupied destinations and retains quarantine. Test cleanup may
remove only its isolated temp root. No production staging, quarantine, restore,
install, cleanup, or deletion API exists.

The schema-1 module process protocol still authorizes no module execution.
Valid plans are unauthorized/unattempted read-only offline previews with exact
receipt-relative executable hashes, public-test-key metadata, cleared
name-allowlisted environments, least-privilege reads, path redaction, and strict
time/I/O ceilings. The only valid module response is `not_executed`; fabricated
success is rejected.

An explicit Cargo feature enables integration tests that copy and execute only
the Cargo-built `rz0-protocol-test-child` under a marked direct OS-temp child.
The test host uses an absolute exact path with no shell, PATH search, or
arguments; clears the parent environment; frames bounded JSON; and concurrently
drains bounded output. Unix test preflight refuses observed non-standard
inheritable descriptors, assigns a new process group, and kills that group on
timeout before reaping the direct helper. This helper is not the inventory
module, is not built by default, is not reachable from the core/CLI/TUI, and is
not a sandbox. Executable replacement/audit races, Windows handle/job control,
and platform capability isolation remain unresolved production blockers.

The schema-1 production execution assessment records the exact canonical gate
set for readiness review but has only a `blocked` decision and requires
`product_execution_authorized: false`. No test, fixture, signature report, or
fully populated assessment can authorize product execution through schema 1.
The opened-artifact identity crate may return a verified same-file handle, but
it contains no spawn function and is not evidence that platform execution is
bound to that handle.

For installed manifests, the loader also verifies explicitly listed files under
the manifest directory with SHA-256. It rejects absolute paths, traversal,
URL-like paths, symlinks, reparse points, files over 64 MiB, and manifests with
more than 128 listed files. This is not an installer, updater, downloader, or
trust decision; it is a local fail-closed check before future install behavior
exists.

`rz0 modules install --dry-run` is not an installer. It validates a local
package directory or manifest, then reports what a future installer would
propose to create, copy, or record. It has no non-dry-run form and must leave
files, PATH, registry, services, scheduled tasks, persistence, and module code
untouched.

The local module store contract is plan-first. `rz0` may calculate user-local
data/state/cache/log/quarantine paths for JSON plans and may initialize only
runtime.zero-owned user-local scaffolding when the user explicitly runs
`rz0 store init --yes`. It must not write module registry entries, module
receipts, transactions, rollback plans, quarantine records, staging files, or
module files until a separate write-capable install gate is explicitly approved.

`rz0 store plan` exposes that same store/routing contract directly for manual
inspection. It is path planning only: no directories, registry files, receipts,
transactions, rollback plans, quarantine records, staging files, module files,
or TUI surfaces are created.

`rz0 store status` is also read-only. It checks whether the future store paths
already exist and reports absent, empty, present, or invalid states. It also
parses an existing `installed-modules.json` registry file if present and reports
registry validity as data. If a valid registry record references a receipt file
that already exists, it validates the receipt shape and cross-checks module
ID/version and store-relative path references. It must not create, repair,
migrate, delete, initialize, trust, activate, or write store files.

`rz0 store status --store-root <path>` uses the same read-only inspection logic
against an explicitly supplied local fixture/support root. The override is not
an installer setting and must not affect module install behavior. Missing roots
are reported as absent and wrong filesystem types are reported as invalid; the
command still must not create, repair, migrate, delete, or write anything.

`rz0 store init --dry-run` is planning only. `rz0 store init --yes` is the only
write-capable foundation command: it may create the runtime.zero user-local
store roots, `state/transactions`, `state/receipts`, an empty schema-1
`installed-modules.json`, and `state/store-init.json`. It must be idempotent,
must refuse to repair or overwrite invalid existing state, and must not install
modules, copy packages, execute code, fetch remote content, edit PATH, create
services/tasks/persistence, or touch credentials, browser profiles, OAuth
sessions, backups, project workspaces, or unknown user data.

Bare `rz0` may open a minimal TUI dashboard in an interactive terminal. That
dashboard is a review surface only: it may display foundation state, store
status, module posture, and safety boundaries, but it must not install, update,
uninstall, repair, execute module code, create store state, or mutate the
system. `rz0 <subcommand>`, JSON output, redirected/piped output,
non-interactive contexts, and `rz0 --no-tui` must stay scriptable and must not
launch the full-screen dashboard.

`rz0 --tui` is an explicit request for the interactive dashboard. It must fail
clearly instead of silently falling back when the terminal is non-interactive or
automation is detected.

The TUI uses crossterm raw key handling and a Ratatui widget renderer for
display only; neither crate path is allowed to create store state, run
modules, or mutate the system. Quit/help/navigation keys do not echo in the
terminal. It treats key press/repeat events as intentional input and ignores key
release events so Windows terminals do not double-advance navigation. Its
navigation model supports visible focus regions, arrows, Tab/Shift+Tab,
`j`/`k`, Enter/Space read-only previews, Esc/back behavior, and Home/End without
changing the safety boundary or executing actions. Its terminal guard must
restore raw mode, cursor
visibility, and the normal screen on exit or panic unwinding. If restoration
fails, that is a TUI runtime bug, not permission to mutate system state.

Color is never required for understanding state. Human-readable output accepts
the global `--color=auto|always|never` flag, respects `NO_COLOR` in auto mode,
and keeps JSON ANSI-free. Status labels remain the authority.

`rz0 scan --dry-run --format json` exposes the empty schema-1 inventory report
contract. The separate `modules/inventory/` workspace package implements bounded
read-only collection without making it part of the core execution surface. It
uses fixture-first PATH normalization, reads process PATH, uses `KEY_READ` for
persisted Windows PATH, detects only allowlisted executable names directly under
PATH entries, and omits hostname/current-user identity and raw registry keys.

Version probes require explicit `--probe-versions`; they invoke an exact
discovered path without a shell, use static version-only arguments, cap captured
output, and kill the child after two seconds. Script-based probes remain
disabled. Platform application evidence requires explicit `--include-apps` and
is bounded. Windows reads standard uninstall views only; macOS enumerates direct
`.app` directories under known roots without reading bundle contents; Linux
reads only bounded XDG desktop-entry files, honors hidden user overrides, and
never emits command lines. Symlinked roots and application records fail closed.
`--redact-paths` replaces path values before sharing. Package-manager
list/update/install/uninstall commands,
network access, credentials/sessions/browser profiles/workspaces/backups/unknown
data, and all writes remain outside the module.

## Local development install boundary

The repository may provide local-only scripts under `scripts/` so Travis can
make the checked-out `rz0` binary available from a normal terminal during
foundation development. That path is intentionally narrow:

- user-local target only, defaulting to `%USERPROFILE%\.local\bin\rz0.exe`;
- user PATH only, and only when the install script is run with `-AddToPath`;
- no administrator requirement or system PATH mutation;
- no remote fetch/download, release publication, package manager install, or
  public direct-run/bootstrap command;
- no shell profile edits, scheduled tasks, services, persistence, module
  install/update/fetch/trust/execution, or real store initialization;
- reversible by `scripts\uninstall-local.ps1 -RemovePath`.

The local install script writes only the copied `rz0.exe` and a
`rz0.local-install.json` marker in the configured install directory. The
uninstall script refuses to remove an unmarked existing target unless `-Force`
is explicitly supplied, and it does not remove a pre-existing PATH entry unless
forced.

## Cleanup risk categories

Future cleanup modules must classify findings before action:

- safe disposable cache;
- stale shims;
- package-manager metadata;
- config/state;
- credentials/session;
- project/workspace data;
- logs/backups;
- unknown.

Only low-risk categories may become eligible for guided quarantine. Credentials/session data, project/workspace data, backups, and unknown findings must remain report-only unless a user explicitly approves a narrowly scoped cleanup.

## Current status

The current CLI/TUI does not include update, uninstall, cleanup, install execution, malware-removal, persistence, or remote module execution behavior. The foundation is limited to read-only diagnostics, a read-only TUI dashboard, the schema-1 inventory contract, a separately built read-only inventory source package, dry-run placeholders, explicit user-local store initialization, dry-run module
install planning, module registry contracts, test-only OS-temp transaction
simulations, an invocation protocol that authorizes no module, and an
explicit-feature Cargo test-helper transport with no core integration.
