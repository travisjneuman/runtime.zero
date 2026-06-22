# Store and Routing Contract

`runtime.zero` now exposes read-only contract plumbing for future local module
state and future CLI/TUI launch routing. This is not an installer and does not
create directories or write state files.

## Local module store contract

The future local module store is user-local by default. Machine-wide installs
are a separate approval gate.

| Platform | Data root | State root | Cache root | Log root | Quarantine root |
| --- | --- | --- | --- | --- | --- |
| Windows | `%LOCALAPPDATA%\runtime.zero` | `%LOCALAPPDATA%\runtime.zero\state` | `%LOCALAPPDATA%\runtime.zero\cache` | `%LOCALAPPDATA%\runtime.zero\logs` | `%LOCALAPPDATA%\runtime.zero\quarantine` |
| macOS | `$HOME/Library/Application Support/runtime.zero` | `$HOME/Library/Application Support/runtime.zero/state` | `$HOME/Library/Caches/runtime.zero` | `$HOME/Library/Logs/runtime.zero` | `$HOME/Library/Application Support/runtime.zero/quarantine` |
| Linux | `${XDG_DATA_HOME:-$HOME/.local/share}/runtime.zero` | `${XDG_STATE_HOME:-$HOME/.local/state}/runtime.zero` | `${XDG_CACHE_HOME:-$HOME/.cache}/runtime.zero` | `${XDG_STATE_HOME:-$HOME/.local/state}/runtime.zero/logs` | `${XDG_STATE_HOME:-$HOME/.local/state}/runtime.zero/quarantine` |

The contract names these future paths:

- `modules_root`;
- `registry_path` for `installed-modules.json`;
- `receipt_path`;
- `transaction_path`;
- `rollback_plan_path`;
- `quarantine_record_path`.

Current code only computes and reports these paths in dry-run output. It does
not create the roots, registry, receipts, transactions, rollback plans,
quarantine records, or module directories.

## Read-only store inspection command

```bash
rz0 store plan
rz0 store plan --format json
```

`rz0 store plan` reports the current platform-specific store contract without
requiring a module package. It reuses the same path-planning primitives used by
the dry-run install planner and reports:

- `store_schema_version`;
- data, state, cache, log, quarantine, and modules roots;
- registry and transaction paths;
- example module, receipt, rollback-plan, and quarantine-record paths;
- rollback and quarantine support flags;
- forbidden path classes;
- CLI/TUI launch-routing interpretation for the current invocation.

The command is read-only. It does not create the roots, registry, receipts,
transactions, rollback plans, quarantine records, staging directories, module
directories, or TUI state.

## Dry-run install metadata

`rz0 modules install --dry-run <package-dir-or-manifest> --format json` includes
a nested `store` object with the future store schema and paths, plus a
`launch_context` object proving the command ran as an explicit scriptable CLI
subcommand.

The store contract includes safety flags:

- `rollback_supported`;
- `quarantine_supported`;
- `forbidden_path_classes`.

Forbidden path classes currently include credentials, browser profiles, OAuth
sessions, unknown user data, backups, and project workspaces. Future rollback
or quarantine behavior must only operate on receipt-listed runtime.zero-owned
paths.

## CLI/TUI routing contract

The future routing contract is deterministic:

- bare `rz0` in an interactive terminal opens the TUI dashboard once the TUI is
  implemented and available;
- bare `rz0` falls back to safe CLI dashboard/status text while TUI is absent;
- `rz0 <subcommand>` always runs the scriptable CLI path;
- `rz0 --json` and `rz0 --format json` never launch a full-screen TUI;
- pipes, redirected output, non-interactive contexts, and automation contexts
  never launch a full-screen TUI;
- a future `--no-tui` bypass forces the scriptable CLI path.

The current slice adds pure routing types and tests only. It does not add a TUI
dependency, TUI renderer, installer, PATH setup, release binary, or bootstrap
command.

## Brand and output constraints

Future CLI/TUI output should follow [`BRAND.md`](../BRAND.md):

- plain labels first, color as reinforcement only;
- JSON output never includes ANSI escape sequences;
- red is only for danger/error/destructive states;
- `[PLAN]`, `[DRY-RUN]`, `[OK]`, `[WARN]`, `[BLOCKED]`, `[ERROR]`, and
  `[QUARANTINE]` remain the preferred status grammar.
