# Uninstall module

`rz0-module-uninstall` owns the shared path-free uninstall finding producer. It
accepts caller-supplied synthetic evidence in its tests and live installed-
software evidence from the foundation catalog. Manager ownership and an exact
installed record are mandatory; protected/unknown evidence remains blocked.

The installed core exposes:

```bash
rz0 uninstall plan <installed-software-id>
rz0 uninstall plan <id> --executable /opt/homebrew/bin/brew --format json
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

The package does not run a manager/uninstaller, resolve dependents/shared
components, consume confirmation, plan direct filesystem writes, elevate,
quarantine, roll back, or execute an uninstall. It has no standalone binary,
signed lifecycle artifact, or production executor. Its manifest remains
`planned`.

Before uninstall can execute, every supported manager/platform needs dependent
and shared-component review, exact executable or bundle identity-to-action
binding, privilege/network policy, manager-native rollback or quarantine-first
restore, cancellation, interruption/power-loss recovery, fresh re-inventory,
CLI/JSON/TUI, and final-artifact acceptance. Protected and unknown software must
remain blocked; direct recursive deletion is not an acceptable shortcut.
