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

The core module-install dry-run now embeds the canonical foundation install
transition instead of maintaining a private lifecycle model. No lifecycle plan
installs, activates, invokes, repairs, migrates, upgrades, deactivates, or removes
anything.
