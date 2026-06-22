# Store and Routing Contract

`runtime.zero` now exposes contract plumbing for local module state and
CLI/TUI launch routing. Store inspection remains read-only; store
initialization is explicit, user-local, and limited to runtime.zero-owned
scaffolding.

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
rz0 store status --store-root <path>
rz0 store status --store-root <path> --format json
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
normal result before `rz0 store init --yes` is explicitly run.

For fixture demos and future support triage, `rz0 store status` also accepts
`--store-root <path>`. The override is read-only and affects only store status
inspection. It does not change module install planning, future install behavior,
or the real user-local store. Missing override roots are reported as absent,
not created. Existing files where a directory root is expected are reported as
invalid. The CLI canonicalizes existing override paths when possible so JSON
output points at the inspected local store root.

### Installed module registry schema

The future installed registry file is:

```text
<state_root>/installed-modules.json
```

Schema version `1` expects:

```json
{
  "schema_version": 1,
  "modules": [
    {
      "id": "first-party.inventory",
      "version": "0.1.0",
      "manifest_path": "modules/first-party.inventory/0.1.0/rz0-module.json",
      "receipt_path": "receipts/rz0plan_inventory.json",
      "module_dir": "modules/first-party.inventory/0.1.0"
    }
  ]
}
```

`module_dir` is optional for forward compatibility with future receipt-first
records, but `id`, `version`, `manifest_path`, and `receipt_path` are required.
Paths are registry references, not instructions to write files. They must be
relative, must not use backslashes, absolute paths, URL-like values, or `..`
traversal, and must stay in the expected future store classes:

- `manifest_path` under `modules/`, ending in `rz0-module.json`;
- `module_dir` under `modules/` when present;
- `receipt_path` under `receipts/`, ending in `.json`.

`rz0 store status` reports registry state as:

- `absent`;
- `empty`;
- `valid`;
- `invalid`;
- `unreadable`.

It also reports schema version, installed module count, duplicate IDs,
malformed record count, unsafe path count, record validation errors, and parser
errors. This is inventory only: a valid registry does not execute, trust,
activate, repair, migrate, install, update, or uninstall any module.

### Install receipt schema

Future install receipts are stored under:

```text
<state_root>/receipts/<receipt-id>.json
```

Registry records reference receipts with a store-relative `receipt_path`, such
as `receipts/rz0plan_inventory.json`. `rz0 store status` only checks receipt
files that are referenced by valid registry records. It does not create missing
receipts or scan unreferenced receipt files.

Schema version `1` expects:

```json
{
  "schema_version": 1,
  "module": {
    "id": "first-party.inventory",
    "version": "0.1.0"
  },
  "source": {
    "source_type": "local_package",
    "package_reference": "fixture-package"
  },
  "target": {
    "module_dir": "modules/first-party.inventory/0.1.0",
    "manifest_path": "modules/first-party.inventory/0.1.0/rz0-module.json"
  },
  "integrity": {
    "manifest_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
    "package_sha256": "1111111111111111111111111111111111111111111111111111111111111111"
  },
  "write_set": [
    {
      "path": "modules/first-party.inventory/0.1.0/rz0-module.json",
      "kind": "manifest",
      "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
      "size_bytes": 1024
    }
  ],
  "rollback": {
    "supported": true,
    "plan_path": "receipts/rz0plan_inventory.rollback.json"
  },
  "quarantine": {
    "supported": true,
    "record_path": "quarantine/modules/rz0plan_inventory/quarantine.json"
  }
}
```

Receipt status is reported as:

- `absent`;
- `valid`;
- `invalid`;
- `unreadable`;
- `unsupported_schema`.

The aggregate receipt state may also be `not_referenced` when no valid registry
records reference receipts. Missing referenced receipts are surfaced as absent
receipt state. Malformed receipts, unsupported receipt schemas, module
ID/version mismatches, unsafe path references, malformed SHA-256 values, and
oversized write-set entries are reported as read-only validation findings.

Receipt paths are references to future runtime.zero-owned state only. Allowed
references are intentionally narrow:

- module target and write-set paths under `modules/`;
- receipt and rollback paths under `receipts/`;
- quarantine records under `quarantine/`.

Absolute paths, backslashes, URL-like values, and `..` traversal are rejected.
Receipt validation is not a trust decision and does not execute, trust,
activate, repair, migrate, install, update, or uninstall any module.

## Store initialization command

Store initialization is implemented as a plan-first foundation command:

```bash
rz0 store init --dry-run
rz0 store init --dry-run --format json
rz0 store init --yes
```

The dry-run command reports the exact filesystem operations that would be
attempted and shares the same JSON/text contract style as `store plan` and
`store status`. The write-capable command requires the explicit `--yes` flag;
plain `rz0 store init` is rejected.

The first write-capable initialization slice only creates runtime.zero-owned
user-local store scaffolding:

- data root;
- state root;
- cache root;
- log root;
- quarantine root;
- modules root;
- transactions directory under state;
- receipts directory under state;
- an `installed-modules.json` registry with schema version `1` and an empty
  `modules` list;
- `state/store-init.json`, a store initialization marker that records schema
  version, created roots, command shape, timestamp, and rollback guidance.

The initialization marker is distinct from module install receipts and does not
imply that any module is installed or trusted. If a registry already exists,
initialization validates it first and refuses to overwrite invalid, empty, or
unreadable state. If `state/store-init.json` already exists, initialization
also validates its schema/kind before treating the store as initialized.

Safety requirements for initialization:

- no remote fetches, module package copies, module execution, PATH edits,
  registry edits, services, scheduled tasks, or persistence hooks;
- no writes outside the computed runtime.zero store roots;
- deny symlinks/reparse points for paths being initialized until a later
  cross-platform policy is approved;
- verify parent directories and permissions before writing;
- be idempotent when all expected paths/files already exist and validate;
- fail closed on partial or mismatched state;
- on failure, report which paths were created and which rollback steps are safe
  for the user to review;
- never delete credentials, browser profiles, OAuth sessions, backups, project
  workspaces, or unknown user data;
- keep uninstall/rollback scoped to runtime.zero-owned files recorded by
  initialization and install receipts.

Repair, migration, uninstall, and module install behavior remain deliberately
blocked for later approval gates.

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

- bare `rz0` in an interactive terminal opens the read-only foundation TUI
  dashboard;
- bare `rz0` falls back to safe CLI dashboard/status text while non-interactive
  or automated;
- `rz0 <subcommand>` always runs the scriptable CLI path;
- `rz0 --json` and `rz0 --format json` never launch a full-screen TUI;
- pipes, redirected output, non-interactive contexts, and automation contexts
  never launch a full-screen TUI;
- `rz0 --no-tui` bypasses the TUI and prints the scriptable text dashboard.

The current TUI shell is intentionally small and dependency-light. It uses
standard-library terminal detection, `crossterm` raw key handling, centralized
theme tokens, a dashboard data model, a resize-safe renderer, and a guarded
event loop. It does not add an installer, PATH setup, release binary, or
bootstrap command.

The TUI key contract is:

- `q` or Esc exits safely without echoing typed input;
- `h` or `?` toggles help;
- Tab/down/right/`j` moves to the next section;
- up/left/BackTab/`k` moves to the previous section;
- Home and End jump to the first and last section;
- terminal resize events re-render the dashboard without changing state;
- key release events are ignored, while press and repeat events remain
  intentional input so Windows terminals do not double-advance navigation.

The terminal guard must restore raw mode, cursor visibility, and the normal
screen on exit or panic unwinding.

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

The current foundation TUI should visually align with the website's
TUI reference as closely as practical by sharing concepts and future design
tokens derived from `BRAND.md`. The website is a visual reference, not a rigid
contract: the TUI must remain easy to customize and refactor, and terminal
usability, accessibility, and safe information hierarchy should win. If the
real TUI evolves into the better source of truth, the website should be updated
afterward to match it.
