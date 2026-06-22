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
- no recursive drive scans;
- bounded manifest file size;
- third-party manifests rejected until the trust model exists.

Validation failure must be reported as data, not repaired automatically.

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

The current CLI does not include update, uninstall, cleanup, install, malware-removal, persistence, or remote module execution behavior. The foundation is limited to read-only diagnostics, dry-run placeholders, and module registry contracts.
