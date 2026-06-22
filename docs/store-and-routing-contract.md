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
rz0 store status
rz0 store status --format json
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

`rz0 store status` inspects the planned paths without initializing them. It
checks data, state, cache, log, quarantine, modules, registry, transactions, and
receipts paths and classifies each path as:

- `absent`;
- `empty`;
- `present`;
- `invalid`.

The overall state is `not_initialized`, `empty`, `present`, or `invalid`.
Type mismatches, metadata errors, and symlinks are reported as invalid state.
This command must remain safe when no store exists yet; an absent store is a
normal result before any write-capable install/store initialization behavior is
approved.

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

- bare `rz0` in an interactive terminal opens the minimal read-only TUI
  dashboard;
- bare `rz0` falls back to safe CLI dashboard/status text while non-interactive
  or automated;
- `rz0 <subcommand>` always runs the scriptable CLI path;
- `rz0 --json` and `rz0 --format json` never launch a full-screen TUI;
- pipes, redirected output, non-interactive contexts, and automation contexts
  never launch a full-screen TUI;
- `rz0 --no-tui` bypasses the TUI and prints the scriptable text dashboard.

The current TUI shell is intentionally small and dependency-free. It uses
standard-library terminal detection, centralized theme tokens, a dashboard data
model, a renderer, and a minimal input loop. It does not add an installer, PATH
setup, release binary, or bootstrap command.

The TUI may show only foundation state: doctor/status posture, store
plan/status summaries, module validation/dry-run posture, safety model, and
future module slots. It must not imply optional modules are installed or active
when the installed module registry is empty.

## Brand and output constraints

Future CLI/TUI output should follow [`BRAND.md`](../BRAND.md):

- plain labels first, color as reinforcement only;
- JSON output never includes ANSI escape sequences;
- red is only for danger/error/destructive states;
- `[PLAN]`, `[DRY-RUN]`, `[OK]`, `[WARN]`, `[BLOCKED]`, `[ERROR]`, and
  `[QUARANTINE]` remain the preferred status grammar.

When the real TUI is implemented, it should visually align with the website's
TUI reference as closely as practical by sharing concepts and future design
tokens derived from `BRAND.md`. The website is a visual reference, not a rigid
contract: the TUI must remain easy to customize and refactor, and terminal
usability, accessibility, and safe information hierarchy should win. If the
real TUI evolves into the better source of truth, the website should be updated
afterward to match it.
