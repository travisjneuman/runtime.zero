# Module System

> For the current source boundary, built-in inventory exception, core-owned
> updater execution lane, per-family maturity, and continuation order, begin
> with [`project-status-and-resumption.md`](project-status-and-resumption.md).

Modules are the unit of growth for `runtime.zero`. The foundation should remain
useful with zero optional modules installed. The end state is a full
system-management platform in which every feature family and provider can be
installed and managed as an independently versioned module. See
[`engineering-handoff.md`](engineering-handoff.md) for the product horizon and
next-shift implementation order.

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

Provider adapters are not automatically lifecycle modules. Each first-party
updater adapter owns only its bounded evidence parser and provider identity;
the shared foundation owns the finding, plan, confirmation, process, receipt,
and recovery boundaries. This keeps one updater lifecycle and one foundation
action path instead of creating duplicate execution stacks. Provider output
with malformed, unterminated, duplicated, or out-of-section evidence stays
unavailable.

## Design rule

Every module must be safe to run in discovery/dry-run mode before it is allowed to mutate anything.

Every module must also be independently manageable. Installed bytes, enabled
state, active execution, and authorization for a particular action are separate
facts. A user may enable inventory without enabling cleanup, enable a specific
AI-tool/provider module without enabling all package managers, or disable a
module while retaining its settings, evidence, and receipts. Disable must stop
module-owned collection, network work, scheduling, UI actions, and mutation;
uninstall is a separate explicit transition with its own data-retention and
rollback review.

Core primitives are not feature modules. `core.cli`, `core.policy`, and
`core.registry` describe the foundation. Optional modules are listed separately
and are not bundled, installed, or executed by default.

## Foundation lifecycle ownership

`crates/module-lifecycle/` owns the only schema-1 transition grammar for install,
activate, invoke, deactivate, repair, migrate, upgrade, and uninstall. Active
modules must deactivate before upgrade or uninstall. Every mutation remains dry-
rollback, and where required process-isolation gates. The bounded v0 executor
uses that grammar for signed `first-party.inventory` on macOS; other module
families remain review/planning-only. Modules supply domain behavior; they must
not implement lifecycle state machines, cancellation engines, registries,
receipts, or transaction coordinators. See
[`module-lifecycle-contract.md`](module-lifecycle-contract.md).

The current CLI exposes the bounded signed v0 path through the foundation and
the seven compiled macOS capabilities through local availability lifecycle
commands. It does not claim configuration, repair, migration,
third-party/remote distribution, or a public package feed. The task-first TUI
uses the same foundation authority; it does not create a second lifecycle
implementation. Remaining target semantics are documented in
[`engineering-handoff.md`](engineering-handoff.md).

## Current registry surface

```bash
rz0 modules
rz0 modules --format json
rz0 modules status
rz0 modules status --store-root tests/fixtures/store-roots/valid-registry-valid-receipt --format json
rz0 modules validate <manifest.json>
rz0 modules --from <directory> --format json
rz0 modules install --dry-run <package-dir-or-manifest>
rz0 modules builtin <install|enable|disable|update|uninstall> --module-id <id> --dry-run|--apply
rz0 modules enable|disable|update|uninstall --module-id first-party.inventory --store-root <path> --dry-run|--apply
rz0 modules recover --recovery-id <id> --store-root <path> --dry-run|--apply
rz0 modules install --developer-trial --dry-run <package-dir-or-manifest> \
  --signature <envelope.json> --trusted-test-key <key.json> --store-root <path>
rz0 modules install --developer-trial --apply <package-dir-or-manifest> \
  --signature <envelope.json> --trusted-test-key <key.json> --store-root <path> \
  --challenge-issued-unix-seconds <seconds> --confirm '<exact phrase>'
rz0 modules install --signed --dry-run <package-dir> \
  --signature <envelope.json> --trusted-test-key <key.json> --store-root <path>
rz0 modules install --signed --apply <package-dir> \
  --signature <envelope.json> --trusted-test-key <key.json> --store-root <path> \
  --challenge-issued-unix-seconds <seconds> --confirm '<exact phrase>'
rz0 modules enable --module-id first-party.inventory --store-root <path> --dry-run
rz0 modules disable --module-id first-party.inventory --store-root <path> --dry-run
rz0 modules update --module-id first-party.inventory --package <package-dir> \
  --signature <envelope.json> --trusted-key <key.json> --store-root <path> --dry-run
rz0 modules uninstall --module-id first-party.inventory --store-root <path> --dry-run
rz0 modules recover --recovery-id <id> --store-root <path> --dry-run
rz0 modules invoke --signed --module-id first-party.inventory --store-root <path> --dry-run
rz0 modules trust verify --manifest <manifest.json> --signature <envelope.json> --trusted-test-key <key.json>
rz0 modules lifecycle-plan install --dry-run --module-id first-party.inventory --from-state absent --to-state installed_inactive --to-version 0.1.0
rz0 store plan
rz0 store plan --format json
rz0 store status
rz0 store status --format json
rz0 store status --store-root tests/fixtures/store-roots/valid-registry-valid-receipt --format json
rz0 store init --dry-run --store-root <path>
rz0 store init --yes --store-root <path>
```

Bare `rz0` opens the Dossier Queue TUI in interactive terminals. It follows
Home → Inventory → Evidence → Plan Review → Confirmation → Activity; module
records remain content inside the attention queue and evidence index. `c`
requests the shared exact-confirmation challenge only from Plan Review.
Explicit subcommands remain the scriptable CLI surface. See
[`tui.md`](tui.md) for the raw-key TUI contract, layout boundaries, and
maintenance boundaries.

The JSON output uses schema version `1` and separates:

- `core`;
- `installed_modules`;
- `planned_module_families`.

An empty `installed_modules` list is valid and expected when no separately
packaged module has been staged. The registry also reports seven
`built_in_capabilities` with local enabled/disabled state. The separate
`planned_module_families` list is the optional package catalog: inventory/
environment, updater, uninstall, leftovers, cache management,
security/integrity, and report/export. Planned package entries are not
installable end-user modules unless their trust and lifecycle slice is marked
available; the signed inventory package is the current exception.

`rz0 modules status` is the operator-facing, path-redacted lifecycle view. It
composes the installed registry, receipt, manifest, and declared package-file
integrity validators and reports an installed record as `installed_inactive` or
`active` only when its persisted foundation state and installed module bytes are
valid; missing, invalid, unreadable, or unsupported evidence produces
`degraded`.
It also reviews developer-only staging receipts and staged destination bytes as
a separate `staged_modules` collection. A valid staged entry is `staged`, not
installed; an invalid receipt or destination is `degraded` in that collection
and remains operator review evidence only. The status review also binds each
staged receipt to its immutable committed transaction-journal head and commit
receipt; missing, tampered, or incomplete transaction evidence is degraded
instead of being presented as a valid staged module.
The bounded macOS `first-party.inventory` record reports activation support and
invocation readiness only after the explicit signed lifecycle path has been
validated. Status itself remains read-only, does not execute module code, and
does not treat an installed record as execution authority.
`--store-root` is a local read-only inspection override for fixtures and
support triage.

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

`rz0 modules install --dry-run <package-dir-or-manifest>` remains a planner for
untrusted or unsigned package review. The bounded signed v0 installer is the
explicit `--signed` form described below.
It accepts a local package directory containing `rz0-module.json`, or a direct
local manifest path, then reuses manifest and package integrity validation. If
the package is valid, it reports proposed install state such as the module
directory, verified files that would be copied later, and the manifest metadata
that would be recorded later. Every planned action has `would_write: false` in
JSON output. The planner itself performs no writes.

The bounded developer trial is the first local write path for module-shaped
bytes, but it is not production installation:

```bash
rz0 modules install --developer-trial --dry-run <package-dir-or-manifest> \
  --signature <envelope.json> --trusted-test-key <key.json> --store-root <path>
```

The dry-run reads a locally selected package, verifies the exact manifest and
declared package files through held file identities, checks a detached public
test-key signature, validates an initialized private store, and prints the
short-lived confirmation phrase. Re-run it with the exact challenge values to
apply. Apply copies only verified bytes into the runtime.zero-owned module
store and records immutable transaction, commit, and developer-stage receipts.
It does not publish `installed-modules.json`, activate or invoke code, fetch
network content, replace an existing version, or establish production trust.
It is a developer fixture lane for validating the foundation write boundary
while production signing, revocation, provenance, sandboxing, upgrade, repair,
rollback, and distribution remain open.

For lifecycle fixture work, the explicit `--developer-promote` flag may be
added to both the dry-run and apply forms. Promotion publishes a single
test-key-only `installed_inactive` registry record and a separate install
receipt through the same commit coordinator. It does not replace an existing
module ID, activate or invoke code, and does not establish production trust.
Without that flag, the developer trial remains staged-only and leaves the
installed registry unchanged.

The signed v0 path is the first end-user lifecycle write lane. It accepts only
the separately packaged, read-only `first-party.inventory` artifact on macOS, verifies a
caller-selected non-revoked `first_party_release` key and exact detached
signature, then uses the foundation store and registry executor:

```bash
rz0 modules install --signed --dry-run <package-dir> \
  --signature <envelope.json> --trusted-test-key <key.json> --store-root <path>
rz0 modules install --signed --apply <package-dir> \
  --signature <envelope.json> --trusted-test-key <key.json> --store-root <path> \
  --challenge-issued-unix-seconds <seconds> --confirm <exact-phrase>
rz0 modules enable --module-id first-party.inventory --store-root <path> --dry-run
rz0 modules disable --module-id first-party.inventory --store-root <path> --dry-run
rz0 modules update --module-id first-party.inventory --package <package-dir> \
  --signature <envelope.json> --trusted-key <key.json> --store-root <path> --dry-run
rz0 modules uninstall --module-id first-party.inventory --store-root <path> --dry-run
rz0 modules recover --recovery-id <id> --store-root <path> --dry-run
```

Each apply form requires the exact challenge values from its current dry-run.
Enable publishes `active`; disable returns to `installed_inactive`; update
requires a newer signed package and keeps it inactive; uninstall moves module
bytes to quarantine before removing the registry record; recover restores only
the recorded quarantine directory. No operation deletes unknown or shared
paths, fetches a package, or runs third-party code.

For the one current lifecycle execution fixture, a promoted inventory package
may be reviewed and invoked through the explicit developer-only process lane:

```bash
rz0 modules invoke --developer-trial --dry-run \
  --module-id first-party.inventory --store-root <path>
rz0 modules invoke --developer-trial --apply \
  --module-id first-party.inventory --store-root <path> \
  --challenge-issued-unix-seconds <seconds> --confirm <exact-phrase>
```

The lane requires a valid promoted registry/receipt, complete immutable
package-file evidence, and a declared Rust inventory executable. It revalidates
the executable through the shared process host and accepts only the
path-redacted read-only inventory contract. It never activates state, writes a
lifecycle receipt, invokes third-party code, or provides production sandbox or
execution authority.

An installed signed v0 package uses the same host through the active lifecycle
state:

```bash
rz0 modules invoke --signed --dry-run \
  --module-id first-party.inventory --store-root <path>
rz0 modules invoke --signed --apply \
  --module-id first-party.inventory --store-root <path> \
  --challenge-issued-unix-seconds <seconds> --confirm <exact-phrase>
```

Successful signed invocation reports `product_execution_authorized: true` only
after executable identity revalidation, bounded process execution, and the
path-redacted read-only inventory response succeed. The host is an identity,
I/O, timeout, and environment boundary, not a native macOS sandbox.

`rz0 modules trust verify` is a separate local package-review command. It
combines exact manifest-byte hashing, declared package-file integrity, and the
detached Ed25519 test-key contract. It does not change the install planner's
status, create a store entry, or authorize any lifecycle transition. Production
keys, provenance, revocation, and execution remain unresolved.

Dry-run JSON now also includes a `store` object and `launch_context` object.
The `store` object describes future user-local data/state/cache/log/quarantine
paths, registry/receipt/transaction paths, rollback/quarantine support flags,
and forbidden path classes. The `launch_context` object records that explicit
subcommands stay on the scriptable CLI path. These are contract fields only:
the command still creates no directories, writes no registry or receipt files,
and launches no TUI.

`rz0 modules lifecycle-plan` exposes the canonical schema-1 transition grammar
without adding a second lifecycle implementation. It requires an operation,
module ID, source state, destination state, and `--dry-run`; versions are
required by the transition grammar when the selected operation needs them. The
output includes the exact ordered foundation gates and a SHA-256 plan digest.
It is useful for review, fixtures, and future TUI/CLI parity, but it does not
publish registry state, consume confirmation, launch module code, or authorize
execution. Invalid transitions, such as upgrading an active module, fail
closed.

See [`manifest-validation.md`](manifest-validation.md) for the validation
contract and current trust boundaries. See
[`store-and-routing-contract.md`](store-and-routing-contract.md) for the local
store and CLI/TUI routing contract, including `rz0 store plan` and
`rz0 store status` for read-only inspection without module install planning,
plus the explicit `rz0 store init --dry-run` / `--yes` scaffold gate.

## First-party module boundary

The foundation has one bounded end-user lifecycle inside a read-only,
first-party boundary: signed `first-party.inventory` packages on macOS may be
staged, published inactive, enabled, disabled, updated, invoked, quarantined,
and recovered. Other modules may rely on manifest validation, SHA-256 package
integrity checks, dry-run install planning, store plan/status,
registry/receipt validation, stable JSON output, and foundation review
surfacing, but they do not gain this executor automatically.

The v0 path does not approve third-party trust, remote fetch, bootstrap/direct-
run commands, cleanup, repair, or broad system mutation. The caller-supplied
release key is a local explicit trust document; production key custody,
provenance, rotation/revocation, public distribution, and native sandboxing
remain open. See [`foundation-readiness.md`](foundation-readiness.md) for the
remaining acceptance gates.

The schema-1 output from `rz0 scan --dry-run --format json` is the live,
path-redacted core inventory contract. The `modules/inventory/` workspace
library supplies fixture-backed and live read-only collectors and is also
embedded as a built-in core adapter. Its source manifest is explicitly
`source_only_module`; a signed package is a separate release artifact and is
not inferred from the embedded adapter. See
[`inventory-schema.md`](inventory-schema.md).

`modules/report-export/` is also a development-only `planned` source package. It
accepts a strict bounded report envelope on stdin and delegates privacy,
validation, digests, bounds, and authority refusal to
`crates/support-contract/`; it owns only report-selection and text/JSON format
behavior. It is not installed or executed by core, while the foundation's
`rz0 report` command calls the same shared builder over redacted live evidence. See
[`support-report-contract.md`](support-report-contract.md).

The remaining five family directories consume `crates/finding-contract/` at
different maturity levels. Updater owns parser/planning behavior used by the
core's separate coordinator; uninstall accepts selected live catalog evidence
and can build a sealed non-authorizing manager plan. Cache now has a bounded
known-root read-only adapter exposed through `rz0 cache --dry-run`; leftovers
has the same bounded adapter over runtime.zero-owned module/log and unreferenced
receipt roots, while
security/integrity remains caller-baseline-only with a bounded exact-file read
adapter. None is a signed/active lifecycle package; the foundation now exposes
one narrow exact-manager uninstall apply boundary, but there is no recursive
cleanup, broad uninstall, or integrity remediation execution. The only other
narrow write exception is one explicitly supplied module-store file through the
confirmation-bound foundation quarantine lane. See
[`domain-classifier-modules.md`](domain-classifier-modules.md).

## Planned module families

- tool/package updater modules;
- manager-native uninstall modules;
- Revo-style leftover scanners;
- cache cleaners;
- environment/PATH inspectors;
- system integrity/security check integrations;
- report/export modules;
- future premium or commercial modules.

These are the initial seven release-gated families, not an exhaustive end-state
catalog. Future independently managed families may cover package/install
provisioning, developer and AI toolchains, services and persistence, storage and
data hygiene, performance/operations, network and connectivity, hardware and
firmware, OS settings, backup/recovery, automation/scheduling, account/provider
integrations, and explicitly separated remote/fleet management. Every addition
must receive a named platform/provider acceptance cell and the same lifecycle,
trust, capability, transaction, privacy, recovery, CLI, JSON, and TUI treatment.

## Trust model

The current implementation executes one optional module path: a signed,
read-only `first-party.inventory` package on macOS after explicit install and
enable. The core also embeds the inventory library as a bounded built-in read
adapter, and owns a separate narrow manager-update executor. The module path
verifies local SHA-256 checksums and detached Ed25519 metadata, binds the exact
declared executable through the foundation process host, clears the child
environment to an allowlist, bounds time/I/O, and validates only the
path-redacted read-only inventory response. A separate fixture-only process
protocol remains available for transport contract tests. The host is not a
native sandbox, and third-party modules remain blocked. Caller-selected release
keys are explicit local trust documents for this v0 path; production key
custody, provenance, rotation/revocation, public distribution, and abuse
handling remain open. The remaining gates are documented in
[`module-trust-and-execution.md`](module-trust-and-execution.md); source
packages do not bypass them.
