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

The current implementation only plans these transitions. The production
executable lifecycle, registry authority, TUI/CLI controls, and disabled-work
guarantee are P0 work for the end-state platform. See
[`engineering-handoff.md`](engineering-handoff.md) for the target command shape
and the shift sequence.

Schema-1 developer promotion now persists the foundation-owned
`lifecycle_state: "installed_inactive"` field in each installed registry
record. The registry accepts no other explicit lifecycle state; this records
verified installation posture and cannot authorize activation, invocation, or a
domain action. Newly generated install receipts carry the same lifecycle state
plus explicit false activation/invocation authority flags; an explicit active
receipt fails receipt validation. Activation remains unavailable until the
production trust, capability, isolation, receipt, and recovery gates are
implemented.

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
