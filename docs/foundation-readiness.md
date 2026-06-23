# Foundation Readiness Gate

This document defines the foundation handoff gate for starting the first first-party module without reopening settled core decisions.

The foundation is ready for first-module planning when the first module stays inside the boundaries below. This is not a public production-ready claim and does not approve module mutation, installation, execution, remote distribution, third-party trust, signing, release automation, or bootstrap/direct-run behavior.

## Complete foundation surfaces

The current foundation provides these module-facing contracts:

- scriptable CLI routing where `rz0 <subcommand>`, JSON, pipes, redirects, and automation contexts never open the full-screen TUI;
- interactive bare-`rz0` TUI routing for safe local review when stdin/stdout are interactive;
- stable foundation dashboard JSON with `schema_version: 1`, `contract: "foundation_dashboard"`, `read_only: true`, and `writes_attempted: false`;
- read-only module manifest validation with capability, risk, lifecycle, dry-run, mutation, rollback, quarantine, and remote-execution fields;
- local SHA-256 package integrity validation for explicitly listed manifest files;
- dry-run-only module install planning that reports proposed state without writing, fetching, trusting, or executing;
- local store contract, `store plan`, `store status`, fixture `--store-root` inspection, installed registry parsing, and receipt validation;
- explicit `store init --dry-run` and `store init --yes` scaffolding limited to runtime.zero-owned user-local store paths;
- read-only Ratatui TUI dashboard with focus regions, command previews, status panels, compact/standard/wide layout tiers, and no command execution from TUI.

## First-module starting boundary

The first first-party module should begin as a read-only module that exercises the contracts above without adding mutation. It may provide local discovery, reporting, fixture-backed validation, dry-run planning, and dashboard/CLI surfacing.

The first module must not:

- install, update, uninstall, repair, clean, or delete anything;
- execute module code, scripts, hooks, WASM, dynamic libraries, package-manager actions, or shell commands beyond already-approved foundation validation commands;
- fetch remote packages or metadata;
- trust third-party authors or package sources;
- mutate PATH, registry, services, tasks, shell profiles, browser profiles, credentials, sessions, backups, unknown user data, or project workspaces;
- publish a release, bootstrap path, direct-run command, signing path, package feed, or automation.

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

- [ ] Module scope is read-only and first-party.
- [ ] CLI output and JSON output are specified before implementation.
- [ ] TUI presentation is a review surface only and does not imply activation.
- [ ] Test fixtures cover valid and fail-closed paths.
- [ ] Safety docs name every blocked mutation/trust boundary.
- [ ] No website, release, bootstrap, package publishing, signing, Cloudflare, GitHub Actions, or external automation change is required.

## Next approved module lane candidate

A safe first candidate is a read-only Windows inventory/discovery module that reports evidence from already-approved sources and emits deterministic text/JSON/TUI summaries. It should start with fixtures and local read-only probes only, then stop for a separate approval before any mutation or installation capability.
