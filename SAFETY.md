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

The future local module store contract is also read-only in the current code.
`rz0` may calculate user-local data/state/cache/log/quarantine paths for JSON
plans, but it must not create those directories or write registry, receipt,
transaction, rollback, quarantine, or staging files until a separate
write-capable install gate is explicitly approved.

`rz0 store plan` exposes that same store/routing contract directly for manual
inspection. It is path planning only: no directories, registry files, receipts,
transactions, rollback plans, quarantine records, staging files, module files,
or TUI surfaces are created.

`rz0 store status` is also read-only. It checks whether the future store paths
already exist and reports absent, empty, present, or invalid states. It also
parses an existing `installed-modules.json` registry file if present and reports
registry validity as data. It must not create, repair, migrate, delete,
initialize, trust, activate, or write store files.

Bare `rz0` may open a minimal TUI dashboard in an interactive terminal. That
dashboard is a review surface only: it may display foundation state, store
status, module posture, and safety boundaries, but it must not install, update,
uninstall, repair, execute module code, create store state, or mutate the
system. `rz0 <subcommand>`, JSON output, redirected/piped output,
non-interactive contexts, and `rz0 --no-tui` must stay scriptable and must not
launch the full-screen dashboard.

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
  install/update/fetch/trust/execution, or future store initialization;
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

The current CLI/TUI does not include update, uninstall, cleanup, install execution, malware-removal, persistence, or remote module execution behavior. The foundation is limited to read-only diagnostics, a read-only TUI dashboard, dry-run placeholders, dry-run module install planning, and module registry contracts.
