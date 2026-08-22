# Foundation Module Lifecycle Contract

`crates/module-lifecycle/` owns module lifecycle transitions so individual
modules contain only domain behavior and cannot invent install, activation,
upgrade, repair, migration, or uninstall state machines.

Schema 1 is planning-only and always requires `dry_run: true`,
`writes_attempted: false`, and `product_execution_authorized: false`. Every plan
has a domain-separated digest and an exact ascending foundation gate set.

Allowed transitions are:

- install: absent → installed/inactive;
- activate: installed/inactive → active;
- invoke: active → active;
- deactivate: active → installed/inactive;
- repair: degraded or installed/inactive → installed/inactive;
- migrate: installed/inactive → installed/inactive at the same version;
- upgrade: installed/inactive → installed/inactive at a different version;
- uninstall: installed/inactive → absent.
- recover: quarantined → installed/inactive at the same version.

Active modules must deactivate before upgrade or uninstall. Every mutation
requires exact artifact identity, capability policy, trust, confirmation,
transaction, and rollback gates; install/repair/upgrade/uninstall also require
process isolation. Invocation is nonmutating at the lifecycle layer but still
requires identity, capabilities, isolation, and trust. Domain action writes use
the separate action-plan and transaction contracts.

## End-state user semantics

The transition grammar is the foundation for a user-selectable module platform.
`installed` means verified bytes and registry state exist; `active` means the
module passed trust, dependency, platform, capability, configuration, and
recovery checks; `disabled` means the module remains installed but performs no
collection, network work, scheduling, UI action, or mutation. `degraded` or
`blocked` means the module is present but a named prerequisite prevents some or
all behavior. A module action being active never authorizes a particular system
mutation: that action still needs its own finding, plan, confirmation,
transaction, and verification.

Disable is an explicit deactivation that preserves module data, settings,
receipts, and evidence. Uninstall is a separate transition that reviews the
exact module-owned write set and data-retention choice; it must refuse shared,
credential, project, backup, and unknown paths. Enable revalidates package
integrity, trust/revocation, dependencies, conflicts, platform support,
effective capabilities, configuration, and pending recovery. Startup must not
implicitly enable, migrate, repair, upgrade, or uninstall a module.

The current v0 implementation executes one bounded end-user path outside the
TUI: a signed, read-only `first-party.inventory` package on macOS. Foundation
execution owns the store, registry, receipts, plan-bound confirmation,
quarantine, and recovery. It does not fetch releases, accept third-party
modules, or imply a general-purpose module sandbox. See
[`engineering-handoff.md`](engineering-handoff.md) for the target command shape
and the remaining release evidence.

## Bounded macOS lifecycle v0

The separately packaged `first-party.inventory` artifact can be installed/staged with a
caller-selected first-party release key, inspected through `modules status`,
enabled, disabled, updated to a newer signed package, invoked through the
foundation process host while active, uninstalled into quarantine, and
recovered from its recorded recovery ID. Every apply command first rebuilds the
current plan and requires the exact five-minute confirmation phrase from its
dry-run response. JSON reports distinguish `read_only`, `writes_attempted`,
`product_execution_authorized`, state transitions, receipts, and exact errors.

The supported command shape is:

```text
rz0 modules install --signed --dry-run <package-dir> --signature <envelope.json> --trusted-test-key <key.json> --store-root <path>
rz0 modules install --signed --apply <package-dir> --signature <envelope.json> --trusted-test-key <key.json> --store-root <path> --challenge-issued-unix-seconds <seconds> --confirm <phrase>
rz0 modules status --store-root <path> --format json
rz0 modules enable --module-id first-party.inventory --store-root <path> --dry-run
rz0 modules disable --module-id first-party.inventory --store-root <path> --dry-run
rz0 modules update --module-id first-party.inventory --package <package-dir> --signature <envelope.json> --trusted-key <key.json> --store-root <path> --dry-run
rz0 modules invoke --signed --module-id first-party.inventory --store-root <path> --dry-run
rz0 modules uninstall --module-id first-party.inventory --store-root <path> --dry-run
rz0 modules recover --recovery-id <id> --store-root <path> --dry-run
```

`--apply` is available for each planned mutating operation with its matching
challenge values. The v0 release-key input is an explicit local trust
document, not a bundled production root: release distribution, key custody,
rotation/revocation, provenance transparency, native sandboxing, and the
other module families remain open.

Developer promotion persists the foundation-owned
`lifecycle_state: "installed_inactive"` field for local test-key staging. The
signed v0 path also publishes `lifecycle_state: "active"` only after an exact
enable confirmation; the registry accepts these two explicit states and no
other arbitrary value. Install receipts still carry explicit false
activation/invocation authority flags. The v0 process host authorizes only the
active, signed macOS inventory package and does not provide a general module
sandbox or third-party execution path.

The core module-install dry-run now embeds the canonical foundation install
transition instead of maintaining a private lifecycle model. No lifecycle plan
installs, activates, invokes, repairs, migrates, upgrades, deactivates, or removes
anything.

The CLI review surface is:

```bash
rz0 modules lifecycle-plan <operation> --dry-run \
  --module-id <module-id> \
  --from-state <state> --to-state <state> \
  [--from-version <version>] [--to-version <version>] \
  [--transition-id <id>] [--format text|json]
```

This is a plan renderer over the crate-owned grammar, not lifecycle execution.
It is intentionally available before the future registry publication and
trust/runtime gates are complete.
