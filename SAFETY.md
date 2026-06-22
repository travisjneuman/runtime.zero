# Safety Policy

`runtime.zero` is a system-management toolkit, so the safety bar is higher than a normal CLI.

## Defaults

- Report first.
- Dry-run first.
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

The current CLI does not include update, uninstall, cleanup, install execution, malware-removal, persistence, or remote module execution behavior. The foundation is limited to read-only diagnostics, dry-run placeholders, dry-run module install planning, and module registry contracts.
