# Foundation Readiness Gate

This document defines the foundation handoff gate for starting the first first-party module without reopening settled core decisions.

The foundation is ready for first-module planning when the first module stays inside the boundaries below. This is not a public production-ready claim and does not approve module mutation, installation, execution, remote distribution, third-party trust, production signing, release automation, or bootstrap/direct-run behavior.

## Complete foundation surfaces

The current foundation provides these module-facing contracts:

- scriptable CLI routing where `rz0 <subcommand>`, JSON, pipes, redirects, and automation contexts never open the full-screen TUI;
- interactive bare-`rz0` TUI routing for safe local review when stdin/stdout are interactive;
- stable foundation dashboard JSON with `schema_version: 1`, `contract: "foundation_dashboard"`, `read_only: true`, and `writes_attempted: false`;
- read-only module manifest validation with capability, risk, lifecycle, dry-run, mutation, rollback, quarantine, and remote-execution fields;
- local SHA-256 package integrity validation for explicitly listed manifest files;
- read-only module permission declarations and a separate detached Ed25519
  verifier constrained to public test keys;
- dry-run-only module install planning that reports proposed state without writing, fetching, trusting, or executing;
- local store contract, `store plan`, `store status`, fixture `--store-root` inspection, installed registry parsing, and receipt validation;
- explicit `store init --dry-run` and `store init --yes` scaffolding limited to runtime.zero-owned user-local store paths;
- read-only Ratatui TUI dashboard with focus regions, command previews, status panels, compact/standard/wide layout tiers, and no command execution from TUI.

The core now emits the empty, versioned `inventory_report` through
`rz0 scan --dry-run --format json`. The first-party `modules/inventory/` source
package targets that contract with bounded read-only collectors, but it remains
neither installed nor executable through the core. TUI content identifies it as
read-only/not installed and previews its separate command without running it.

## First-module starting boundary

The first first-party module has begun as the separate read-only `modules/inventory/` source package. It exercises the contracts above through local discovery, reporting, fixture-backed validation, optional bounded probes, and preview-only TUI/CLI surfacing without adding core module execution or mutation.

The first module must not:

- install, update, uninstall, repair, clean, or delete anything;
- execute module code, scripts, hooks, WASM, dynamic libraries, package-manager actions, or shell commands beyond already-approved foundation validation commands;
- fetch remote packages or metadata;
- trust third-party authors or package sources;
- mutate PATH, registry, services, tasks, shell profiles, browser profiles, credentials, sessions, backups, unknown user data, or project workspaces;
- publish a release, bootstrap path, direct-run command, production signing
  path, package feed, or automation.

## Module-facing invariants

A first-party module can rely on these invariants:

- core output stays text-first, label-first, and color-optional;
- JSON contracts are additive and versioned;
- TUI content mirrors existing dashboard/module/store state and remains read-only;
- dry-run reports must disclose proposed writes with `would_write: false` until a separate approval enables writes;
- local file paths must remain under declared module/store roots and must reject traversal, absolute package paths, URL-like paths, symlinks, reparse points, unsafe receipts, and unsupported integrity algorithms;
- installed-module registry and receipts are evidence surfaces, not trust or activation decisions;
- third-party trust remains blocked.

## Acceptance checklist before module implementation starts

- [x] Module scope is read-only and first-party.
- [x] CLI output and JSON output are specified before implementation.
- [x] TUI presentation is a review surface only and does not imply activation.
- [x] Test fixtures cover valid, duplicate, missing, malformed, invalid-entry,
  and unsupported-platform paths.
- [x] Safety docs name every blocked mutation/trust boundary.
- [x] No website, release, bootstrap, package publishing, production signing,
  Cloudflare, GitHub Actions, or external automation change is required.

## Current handoff outcome

The inventory source package satisfies this handoff gate. It remains planned and
uninstalled, and real Windows runtime proof is still required. Read-only
permission validation, test-key-only detached signature verification, and
OS-temp-root staging/quarantine/restore simulations are now implemented. The
fixture-only invocation protocol is also implemented without module execution.
An explicit-feature Cargo test-helper lane now proves bounded framing,
environment/cwd setup, concurrent drains, and direct-child timeout kill/reap on
the native development host. The next trust lane is executable-handle pinning,
process-tree/handle control, and real Windows/macOS/Linux capability-isolation
proof; production mutation, installation, signing, release, and core module
execution remain separate approval gates.
