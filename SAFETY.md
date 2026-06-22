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

## Phase 1 status

Phase 1 does not include update, uninstall, cleanup, install, malware-removal, or persistence behavior. The initial CLI is limited to read-only diagnostics and dry-run placeholders.
