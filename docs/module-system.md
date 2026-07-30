# Module System

Modules are the unit of growth for `runtime.zero`. The foundation should remain useful with zero optional modules installed.

A module manifest declares:

- manifest version;
- id and display name;
- version and publisher;
- module kind;
- lifecycle status;
- supported platforms;
- capabilities;
- risk level;
- whether it mutates the system;
- confirmation requirements;
- dry-run requirements;
- quarantine/rollback support;
- remote execution policy;
- optional local package integrity metadata;
- test fixtures.

## Design rule

Every module must be safe to run in discovery/dry-run mode before it is allowed to mutate anything.

Core primitives are not feature modules. `core.cli`, `core.policy`, and
`core.registry` describe the foundation. Optional modules are listed separately
and are not bundled, installed, or executed by default.

## Foundation lifecycle ownership

`crates/module-lifecycle/` owns the only schema-1 transition grammar for install,
activate, invoke, deactivate, repair, migrate, upgrade, and uninstall. Active
modules must deactivate before upgrade or uninstall. Every mutation remains dry-
run-only and binds identity, trust, capability, confirmation, transaction,
rollback, and where required process-isolation gates. Modules supply domain
behavior; they must not implement lifecycle state machines, cancellation
engines, registries, receipts, or transaction coordinators. See
[`module-lifecycle-contract.md`](module-lifecycle-contract.md).

## Current registry surface

```bash
rz0 modules
rz0 modules --format json
rz0 modules validate <manifest.json>
rz0 modules --from <directory> --format json
rz0 modules install --dry-run <package-dir-or-manifest>
rz0 store plan
rz0 store plan --format json
rz0 store status
rz0 store status --format json
rz0 store status --store-root tests/fixtures/store-roots/valid-registry-valid-receipt --format json
rz0 store init --dry-run
```

Bare `rz0` opens a read-only foundation TUI dashboard shell in interactive
terminals. That dashboard may show module posture, the installed count, planned
first-party families, and future module slots, but it must not claim planned
module families are installed or executable. Explicit subcommands remain the
scriptable CLI surface. See [`tui.md`](tui.md) for the raw-key TUI contract,
layout boundaries, and maintenance boundaries.

The JSON output uses schema version `1` and separates:

- `core`;
- `installed_modules`;
- `planned_module_families`.

An empty `installed_modules` list is valid and expected for the foundation-only
build. The planned registry is pinned by test to the frozen seven-family 1.0
catalog: inventory/environment, updater, uninstall, leftovers, cache management,
security/integrity, and report/export. Planned entries are not implementations.

`rz0 modules validate` reads one local JSON manifest and reports whether it
passes the foundation contract. `rz0 modules --from <directory>` reads JSON
manifests directly inside that directory and includes only valid manifests in
`installed_modules`. Neither command executes code or fetches remote content.

The future installed-module registry lives at the store contract's
`installed-modules.json` path. `rz0 store status` can now parse that file if it
already exists and report whether it is absent, empty, valid, invalid, or
unreadable. Registry parsing is read-only and does not make a trust or
activation decision. If a valid registry record references an existing receipt
file, `store status` also validates that receipt and checks that receipt
module/version metadata matches the registry record.

For demos and support triage, `rz0 store status --store-root <path>` can inspect
a supplied local fixture/store root with the same parser and validator. The
override is intentionally limited to read-only store inspection; it does not
initialize state, write registry/receipt files, or change future install
behavior.

`rz0 store init --dry-run` reports the future store scaffolding plan.
`rz0 store init --yes` may initialize only runtime.zero-owned user-local store
scaffolding; it does not install modules, activate registry records, trust
receipts, or execute module code.

Installed manifests are valid only when their explicitly listed package files
pass local SHA-256 integrity checks. Planned manifests may omit integrity
metadata, but the validator reports that they are not package-verified yet.
The first integrity slice supports only local directory packages rooted at the
manifest directory; it rejects absolute paths, traversal, URLs, symlinks,
reparse points, files over 64 MiB, and manifests with more than 128 listed
files.

`rz0 modules install --dry-run <package-dir-or-manifest>` is a planner only.
It accepts a local package directory containing `rz0-module.json`, or a direct
local manifest path, then reuses manifest and package integrity validation. If
the package is valid, it reports proposed install state such as the module
directory, verified files that would be copied later, and the manifest metadata
that would be recorded later. Every planned action has `would_write: false` in
JSON output. The command performs no writes and intentionally has no non-dry-run
form.

Dry-run JSON now also includes a `store` object and `launch_context` object.
The `store` object describes future user-local data/state/cache/log/quarantine
paths, registry/receipt/transaction paths, rollback/quarantine support flags,
and forbidden path classes. The `launch_context` object records that explicit
subcommands stay on the scriptable CLI path. These are contract fields only:
the command still creates no directories, writes no registry or receipt files,
and launches no TUI.

See [`manifest-validation.md`](manifest-validation.md) for the validation
contract and current trust boundaries. See
[`store-and-routing-contract.md`](store-and-routing-contract.md) for the local
store and CLI/TUI routing contract, including `rz0 store plan` and
`rz0 store status` for read-only inspection without module install planning,
plus the explicit `rz0 store init --dry-run` / `--yes` scaffold gate.

## First first-party module boundary

The foundation is ready for first-module planning only inside a read-only,
first-party boundary. The first module may rely on manifest validation,
SHA-256 package integrity checks, dry-run install planning, store plan/status,
registry/receipt validation, stable JSON output, and read-only TUI surfacing.

Starting module work does not approve module execution, real install/update/
uninstall behavior, third-party trust, production signing, release/package publishing,
remote fetch, bootstrap/direct-run commands, cleanup, repair, or broad system
mutation. See [`foundation-readiness.md`](foundation-readiness.md) for the
handoff gate and acceptance checklist.

The schema-1 output from `rz0 scan --dry-run --format json` is the core inventory
contract. The `modules/inventory/` workspace package now targets it with
fixture-backed and live read-only collectors while remaining absent from the
core dependency and execution paths. Its manifest stays `planned`: it is source
for development, not a published/installed artifact. See
[`inventory-schema.md`](inventory-schema.md).

`modules/report-export/` is also a development-only `planned` source package. It
accepts a strict bounded report envelope on stdin and delegates privacy,
validation, digests, bounds, and authority refusal to
`crates/support-contract/`; it owns only report-selection and text/JSON format
behavior. It is not installed or executed by core. See
[`support-report-contract.md`](support-report-contract.md).

## Planned module families

- tool/package updater modules;
- manager-native uninstall modules;
- Revo-style leftover scanners;
- cache cleaners;
- environment/PATH inspectors;
- system integrity/security check integrations;
- report/export modules;
- future premium or commercial modules.

## Trust model

The initial implementation does not execute optional modules. First-party
modules should later be signed and explicitly installed or enabled. The
foundation verifies local SHA-256 checksums, and a separate workspace contract
can verify detached Ed25519 signatures against caller-selected public test keys.
That test-only verifier is not integrated with installation and does not make a
production or network trust decision. A separate fixture-only process protocol
requires exact receipt metadata, least-privilege read grants, a cleared bounded
environment allowlist, and a `not_executed` module response. An explicit Cargo
feature launches only a Cargo-built test helper under guarded OS-temp roots to
exercise transport failure behavior; no inventory module or core execution path
is implemented. Third-party modules are expected eventually, but only
after a hardened trust model covering signing, provenance, sandboxing,
permissions, revocation, and abuse cases. The required staged gate is documented
in [`module-trust-and-execution.md`](module-trust-and-execution.md); current
source packages do not bypass it.
