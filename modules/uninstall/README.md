# Uninstall module

`rz0-module-uninstall` owns the shared path-free uninstall finding producer. It
accepts caller-supplied synthetic evidence in its tests and live installed-
software evidence from the foundation catalog. Manager ownership and an exact
installed record are mandatory; protected/unknown evidence remains blocked.

The installed core exposes:

```bash
rz0 uninstall plan <installed-software-id>
rz0 uninstall plan <id> --executable /opt/homebrew/bin/brew --format json
rz0 uninstall apply <id> --executable /opt/homebrew/bin/brew --accept-no-rollback
```

The command converts one live record into the module's
`classified_finding_report`. For manager-owned software it also attempts a
finding-bound shared action plan. The action is blocked unless an allowlisted
direct manager artifact is observed and its SHA-256/size identity is sealed into
the plan. A successfully sealed action can be `planned`, but remains dry-run,
`writes_attempted: false`, and `execution_authorized: false`.

Current dispositions:

- protected system software: blocked;
- supported manager-owned software: manager-native review/action plan;
- user/local bundles: quarantine-first report-only pending an exact root-
  relative mover/restore contract;
- package-receipt-only or unknown ownership: unsupported/blocked.

The package does not own a manager process, resolve dependents/shared
components, plan direct filesystem writes, elevate, quarantine, or roll back.
The foundation now exposes one narrow manager-native apply lane for exact
manager-owned records. It consumes the shared destructive confirmation,
requires `--accept-no-rollback` because manager rollback is not yet proven,
records the external effect through the canonical transaction/receipt path, and
requires fresh installed-software evidence to omit the target. Protected,
user-owned, unknown, and non-manager records remain report-only. The module has
no standalone binary, signed lifecycle artifact, or production package
executor; its manifest remains `planned`.

The current apply lane is pre-alpha and should be treated as a manager-native
transaction proof point, not a 1.0 claim. Before uninstall is supported broadly,
every manager/platform needs dependent and shared-component review, exact
executable or bundle identity-to-action binding, privilege/network policy,
manager-native rollback or quarantine-first restore, signal cancellation,
interruption/power-loss recovery, fresh re-inventory, TUI/accessibility, and
final-artifact acceptance. Protected and unknown software must remain blocked;
direct recursive deletion is not an acceptable shortcut.
