# Uninstall module

`rz0-module-uninstall` is a development-only manager-native uninstall finding
classifier. It accepts caller-supplied synthetic records and emits the shared
path-free finding contract only when installed evidence and manager ownership
are both present.

The package does not discover software, run a manager/uninstaller, resolve
dependents or shared components, plan writes, elevate, quarantine, roll back, or
execute an uninstall. It has no binary, live host permissions, signed lifecycle
artifact, or core integration. Its manifest remains `planned`.

The installed core separately exposes `rz0 uninstall plan <id>` for live Mac
catalog records. That command returns a non-executing `uninstall_review`; it is
not this module's finding report or a shared executable action plan.

Before uninstall can execute, live catalog evidence must bind through the shared
finding/action/confirmation/transaction pipeline. Every supported manager and
platform also needs dependent/shared-component review, exact executable or
bundle identity, privilege/network policy, manager-native rollback or
quarantine-first restore, cancellation, interruption/power-loss recovery, fresh
re-inventory, CLI/JSON/TUI, and final-artifact acceptance. Protected and unknown
software remains blocked; direct recursive deletion is not an acceptable
shortcut.
