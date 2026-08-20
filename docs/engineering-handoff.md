# Engineering Handoff: Modular Full-System Management

> Reviewed 2026-08-17. This is the shift-handoff document for the current
> `runtime.zero` repository. It explains the product end state, the architecture
> required to reach it, what is actually working today, and the order in which
> the next engineering shift should continue. It is intentionally explicit
> about the difference between a product direction, a checked-in contract, and
> a working end-user feature.

## Read this first

`runtime.zero` is active pre-alpha development. It is not a supported release,
and the current module lifecycle is still planning-only. The current source
snapshot is the `main` branch at the updater/TUI implementation commit recorded
in [`project-status-and-resumption.md`](project-status-and-resumption.md).

The current product already has a real primary TUI, a scriptable CLI, bounded
inventory and monitoring, module manifests and registry inspection, and a
provider-driven macOS/Linux updater lane. The TUI `U` action and CLI updater
apply path share one confirmation, identity, transaction, receipt, and
verification path. That is useful pre-alpha behavior, not proof that every
software source on every operating system is covered.

The next shift should use this document for direction, the current-status guide
for facts, the narrow contracts for implementation rules, and the safety and
security documents for non-negotiable boundaries:

1. [`project-status-and-resumption.md`](project-status-and-resumption.md) —
   current behavior, evidence, gaps, and restart order;
2. [`architecture.md`](architecture.md) — foundation/module/platform ownership;
3. [`module-system.md`](module-system.md) — module manifest and registry model;
4. [`module-lifecycle-contract.md`](module-lifecycle-contract.md) — the
   foundation-owned lifecycle state machine;
5. [`module-trust-and-execution.md`](module-trust-and-execution.md) — package
   trust, capabilities, isolation, and execution gates;
6. [`production-readiness.md`](production-readiness.md) and
   [`completion-checklist.md`](completion-checklist.md) — release evidence;
7. [`SAFETY.md`](../SAFETY.md) and [`SECURITY.md`](../SECURITY.md) — rules that
   no feature or module may weaken.

## The end game

The end state is a full system-management platform for a local machine, with
optional expansion to explicitly governed remote or fleet modules later. It is
not a single updater with a long list of hard-coded commands, and it is not a
monolithic cleaner that owns every feature forever.

The product promise is:

> Every meaningful system-management capability is an independently versioned,
> inspectable module. Users can install, enable, configure, disable, update,
> repair, or uninstall each module according to their use case. A small,
> stable foundation gives every module the same identity, evidence, policy,
> capability, TUI, CLI, JSON, transaction, rollback, privacy, and recovery
> behavior.

In the finished product, a user might enable only inventory and monitoring on a
work computer; enable package and AI-tool updates on a development Mac; add
storage cleanup and backup/recovery on a personal machine; or run a tightly
restricted security and service-audit profile on a server. The foundation must
not require every feature, provider, daemon, network integration, or privileged
operation to be installed or active for the basic product to work.

“Full system management” is a scope horizon, not a claim that every imaginable
feature belongs in the first release. Each capability must enter the explicit
module/platform/provider acceptance ledger before it becomes a support claim.
The first seven module families are the initial release gate; they are not the
ceiling of the product.

## Product surface at the end state

The foundation provides one coherent control surface:

- the TUI is the primary interactive workflow for discovery, review, enablement,
  configuration, action, progress, cancellation, recovery, and module state;
- the CLI exposes the same operations for scripting, automation, support, and
  headless systems;
- JSON is a stable machine contract for every report, finding, plan, action,
  receipt, state transition, and unsupported result;
- module contributions appear in the same navigation, details, action, status,
  help, and recovery vocabulary rather than creating competing mini-apps;
- the foundation owns global search, filtering, permissions, audit history,
  notifications, diagnostics, and the distinction between evidence, a plan,
  an approved action, and an external effect.

The TUI and CLI are equally important authorities. The TUI is not a deprecated
visual shell and the CLI is not a separate implementation. They must call the
same foundation services and the same module action contracts. A capability is
not complete if it only exists in a parser, fixture, background worker, or
developer-only command.

## Capability horizon

The following is the proposed long-term catalog. It is a planning map, not a
claim that these modules are implemented or already committed to 1.0. Each
family can contain multiple independently versioned modules, and a provider or
platform adapter may be split into its own module when it has a separate trust,
release, or permission boundary.

| Capability family | Examples of modules that may live here |
| --- | --- |
| Inventory and identity | Applications, packages, binaries, versions, publishers, receipts, PATH, environments, drivers, services, persistence, hardware, and source disagreement |
| Package and software updates | Homebrew, MacPorts, Apple Software Update, App Store tooling, APT/dpkg, DNF/RPM, pacman, Snap, Flatpak, Winget, Chocolatey, Scoop, Nix, language managers, and app-specific channels |
| Developer and AI toolchains | npm prefixes, pip, Cargo/crates.io, rustup, uv, Deno, Codex, Grok, Hermes, OMP/oh-my-pi, Pi, GSD, T3 Code, Kilo, Warp, and other tool-specific or editor-specific providers when their real update contract is known |
| Installation and provisioning | Manager-native installation, package/source selection, manifests, local packages, toolchain profiles, and reproducible environment setup |
| Uninstall and cleanup | Manager-native uninstall, dependency review, shared-component review, leftovers, shims, receipts, caches, and quarantine/restore |
| Storage and data hygiene | Disk inventory, cache management, logs, temporary files, duplicate review, large-file review, filesystem health, and bounded cleanup |
| Services and persistence | Launch agents, launch daemons, systemd, Windows services/tasks, login items, startup entries, scheduled work, and persistence review |
| Performance and operations | CPU, memory, disk, network, process, thermal, battery, resource pressure, diagnostics, alerts, and bounded local monitoring |
| Security and integrity | File/config baselines, signature and publisher checks, permission/ACL review, drift, vulnerability evidence, incident guidance, and integrity verification |
| Privacy and data lifecycle | Local data classification, report redaction, browser/profile review, telemetry settings, data retention, export review, and explicit destruction workflows |
| Network and connectivity | Firewall, DNS, proxy, VPN, routes, ports, certificates, connectivity diagnostics, and network policy; every privileged or remote action needs its own grant |
| Hardware, drivers, and firmware | Device inventory, driver state, firmware evidence, battery health, displays, peripherals, and vendor-native update/recovery paths |
| OS settings and policy | Updates, permissions, defaults, power, security posture, shell/environment settings, user-local policy, and platform-native configuration |
| Backup and recovery | Snapshots, backup targets, restore review, recovery media, rollback, disaster-recovery checks, and interrupted-transaction recovery |
| Automation and scheduling | User-authored routines, reminders, scheduled scans, notifications, and event-triggered actions with explicit opt-in and visible ownership |
| Accounts and secrets | Provider-scoped account/session inventories, credential expiry, keychain/credential-manager integrations, and rotation guidance; no broad secret cleanup |
| Remote and fleet management | Explicitly separate remote hosts, agents, inventory, policy, and fleet orchestration modules; never smuggle remote authority into a local module |

This catalog deliberately includes difficult or sensitive areas. Their inclusion
means the architecture must be able to host them safely; it does not grant
authority to implement them casually. Credentials, browser profiles, user
content, backups, private projects, remote hosts, and unknown data remain
blocked until a separately reviewed module contract exists.

## What “universal” means

Universal coverage is an evidence obligation, not a promise to guess. For every
installed or discovered item, runtime.zero should:

1. preserve the source-native identity and provider that found it;
2. resolve the exact executable, package, app bundle, receipt, channel, or
   service owner before proposing an action;
3. use a provider adapter with a documented query, update/install/uninstall
   contract, version and locale behavior, privilege model, network behavior,
   and rollback/recovery story;
4. show `supported`, `delegated`, `observed_only`, `missing`, `unsupported`,
   `blocked`, or `unavailable` explicitly when an action cannot be proven;
5. never turn a PATH name, display-name match, or generic download URL into an
   executable update command;
6. add every supported provider/platform combination to the acceptance ledger.

The current provider lane already demonstrates this shape for several macOS
and Linux sources. It can find tools such as Codex, Pi, GSD, and other npm-owned
CLIs when their actual prefix is present, and it has dedicated provider logic
for Grok, Hermes, OMP, and Warp where the executable/channel contract permits
it. T3 Code-style Electron/Squirrel metadata and Sparkle observations are
different channel classes. A future shift must preserve those distinctions
instead of advertising one “all apps” switch as proof of universal coverage.

## Module contract

Every end-state module is a product component, not just a Rust crate. A module
must provide or consume the following foundation-owned contract pieces.

### Identity and package

- immutable module ID, publisher, version, schema versions, and build target;
- complete-file manifest, package digest, provenance, signature, release channel,
  freshness, revocation state, and source release identity;
- supported OS/platform/architecture/provider matrix and explicit exclusions;
- declared dependencies, conflicts, replacement/upgrade rules, and migration
  versions;
- resource, privacy, data-retention, and network declarations.

### Capability and policy

- an explicit least-privilege capability request;
- read, network, manager, state-write, quarantine, restore, elevation, account,
  remote, and destructive capability separation;
- an effective policy view supplied by the foundation, never a private policy
  parser that can expand authority;
- visible risk, permission, confirmation, and rollback requirements;
- a refusal/unsupported path for every platform or provider it cannot prove.

### Domain behavior

- bounded discovery and source-status evidence;
- normalized findings with immutable evidence digests and source provenance;
- dry-run plans with exact target, provider, executable, arguments, network,
  privilege, write set, expected before/after state, expiry, and rollback;
- action execution only through foundation services;
- fresh post-action verification and typed partial/failure/recovery outcomes;
- no private transaction, receipt, logger, scheduler, lifecycle, or trust stack.

### User and machine surfaces

- CLI subcommands and help;
- JSON schema and stable machine errors;
- TUI section/detail/action/status/help contributions;
- configuration schema, defaults, migration, and reset behavior;
- audit events, receipts, support diagnostics, and privacy-reviewed exports;
- tests for keyboard, mouse, no-color, compact layout, accessibility, and
  non-interactive operation where the capability is exposed.

## Enable, disable, and uninstall semantics

The user-facing module model must distinguish package presence from activation:

~~~text
not installed -> installed/disabled -> enabled/active -> degraded/blocked
                         ^                |                 |
                         |                v                 v
                    disabled <-------- deactivate       repair/review
                         |
                         v
                  explicit uninstall
~~~

The exact persisted state names may evolve with the reviewed schema, but the
semantics must remain stable:

- **Installed** means verified module bytes and manifest state exist locally.
- **Disabled** means the module is retained but contributes no collection,
  scheduled work, background process, network activity, TUI action, or system
  mutation. Its installed metadata and prior receipts remain inspectable.
- **Enabled/active** means the module passed trust, platform, dependency,
  capability, configuration, and lifecycle checks and may contribute its
  declared read surfaces. A specific mutation still requires its own plan and
  confirmation.
- **Degraded/blocked** means the module remains present but a dependency,
  permission, provider, migration, integrity, platform, or recovery condition
  prevents some or all behavior. The TUI and CLI must explain the condition and
  the recovery path.
- **Disable** is an explicit lifecycle action. It first stops or unregisters
  module-owned recurring work through the foundation, preserves state and
  receipts, and does not delete user data or module data.
- **Uninstall** is separate from disable. It verifies the exact package and
  receipt-owned write set, offers data-retention choices, uses quarantine or
  rollback where applicable, and refuses shared, credential, project, backup,
  or unknown paths.
- **Enable** revalidates package integrity, trust/revocation, dependencies,
  conflicts, platform support, effective capability grants, configuration, and
  pending recovery before activation. Enabling never silently updates the OS or
  enables a second module without an explicit dependency decision.
- **Upgrade/repair/migrate** are separate transitions with receipts, version
  compatibility, interruption handling, and rollback/recovery. Startup must
  never perform them implicitly.

The target command shape is illustrative, not current CLI behavior:

~~~text
rz0 modules list --all
rz0 modules inspect <module-id>
rz0 modules install <package> --dry-run
rz0 modules lifecycle-plan <operation> --dry-run --module-id <id> --from-state <state> --to-state <state>
rz0 modules enable <module-id> --dry-run
rz0 modules enable <module-id> --confirm <exact-phrase>
rz0 modules disable <module-id> --dry-run
rz0 modules disable <module-id> --confirm <exact-phrase>
rz0 modules configure <module-id> --dry-run
rz0 modules update <module-id> --dry-run
rz0 modules repair <module-id> --dry-run
rz0 modules uninstall <module-id> --dry-run
~~~

These commands must not be added as cosmetic aliases before the lifecycle
runtime, registry publication, trust, configuration, receipts, recovery, and
TUI flows can support them. The current repository exposes listing, validation,
store inspection, and installation planning only; it does not yet expose these
mutating module lifecycle commands.

The current `modules lifecycle-plan` command is a bounded review renderer for
the crate-owned schema-1 transition grammar. It is not one of the mutating
commands above: it does not publish state, consume confirmation, execute
module code, or authorize a transition.

## Architecture required to reach the end state

The foundation remains intentionally small and owns the cross-cutting control
plane:

1. **Foundation kernel:** CLI/TUI/JSON routing, configuration, diagnostics,
   logging, error taxonomy, compatibility, permissions, and module discovery.
2. **Module store and registry:** immutable package staging, verified manifests,
   installed/disabled/active state, dependencies/conflicts, settings, receipts,
   migration, rollback, and recovery.
3. **Policy and capability broker:** effective grants, network/elevation rules,
   resource ceilings, privacy classes, and OS-level enforcement.
4. **Process and provider host:** exact executable identity, environment/cwd,
   bounded transport, cancellation, containment, provider selection, and
   platform adapters.
5. **Evidence/action engine:** source status, findings, plans, confirmation,
   transactions, external-effect receipts, verification, quarantine, rollback,
   and recovery.
6. **Modules:** domain logic and provider-specific behavior only. Modules consume
   foundation services and contribute manifests, schemas, evidence, actions,
   views, and tests.
7. **Surfaces:** the TUI, CLI, JSON, support reports, and optional future API
   render the same state and invoke the same action contracts.

No module may fork the transaction model, invent a second confirmation prompt,
launch through PATH search, silently broaden network access, keep a private
registry, or create a background service without a declared and approved
capability. The foundation must also remain useful when all optional modules are
disabled.

## Delivery order for the next shifts

### P0 — make the module platform real

1. Turn the planning-only lifecycle grammar into a foundation-owned executable
   lifecycle with immutable staging, registry publication, receipts, rollback,
   recovery, and explicit confirmation.
2. Define the production module package/signing/provenance/revocation policy and
   implement the capability broker and isolated module host on Windows, macOS,
   and Linux.
3. Add installed/disabled/enabled/degraded state and module settings to the
   store, JSON, CLI, and primary TUI. Make disable behavior observable and
   ensure disabled modules do no work.
4. Define the module contribution API so a new feature adds a module rather than
   new foundation-specific branches throughout `src/`.
5. Build a provider/source ledger that enumerates every supported manager,
   app-update channel, language environment, AI tool, service, and platform
   combination with explicit gap states.

### P1 — complete the initial release platform

1. Finish Windows/macOS/Linux identity binding, process containment, capability
   enforcement, network/elevation rules, cancellation, recovery, rollback, and
   disposable-host proof for the current updater exception.
2. Advance the initial seven families — inventory/environment, updater,
   uninstall, leftovers, cache, security/integrity, and report/export — through
   the same signed lifecycle, TUI/CLI/JSON, platform, accessibility, and
   release-ledger requirements.
3. Complete source parity and reconciliation for the explicitly frozen package,
   app, service, persistence, and toolchain providers. Preserve observed-only
   and unsupported results where a safe action cannot be proven.
4. Add user configuration, migrations, per-module schedules, notifications,
   retention, and repair UX without making background work the default.

### P2 — expand the full system-management catalog

Add the capability families in the catalog above as independently reviewed
modules. Prioritize storage/data hygiene, services/persistence, developer/AI
toolchains, security/integrity, backup/recovery, network diagnostics, hardware,
OS settings, automation, and account/provider integrations according to user
need and available platform contracts. Do not expand a family by copying an
unsafe updater or cleanup shortcut into a new module.

### P3 — ecosystem and remote management

Only after local module trust, lifecycle, recovery, and support are mature,
consider third-party module distribution, publisher governance, remote hosts,
fleet policy, shared catalogs, and signed automation. Remote authority is a
separate threat model and must not be implied by local module enablement.

## Definition of done for a module

A module is complete only when all of the following are true for every claimed
platform/provider cell:

- requirements, non-goals, threat model, privacy classes, and ownership are
  documented;
- its immutable signed artifact, manifest, dependencies, and capabilities are
  validated;
- install, enable, invoke, disable, upgrade, repair, migrate, and uninstall
  transitions have receipts, interruption behavior, rollback, and recovery;
- discovery is bounded, deterministic, source-specific, redacted, and honest
  about partial/unavailable evidence;
- every action has a finding, exact plan, expiry, confirmation, transaction,
  expected state, fresh verification, and typed failure/recovery outcome;
- the CLI, JSON, TUI, help, diagnostics, support output, and accessibility
  behavior are implemented from the same contract;
- capabilities, network, elevation, process, filesystem, resource, and data
  boundaries are enforced at runtime, not just declared in JSON;
- adversarial, fault, race, power-loss, cancellation, locale, filesystem,
  terminal, and platform tests produce final-artifact evidence;
- the module is documented in the catalog, release ledger, user guide, and
  troubleshooting/recovery material;
- disabling it is reversible and quiet, while uninstalling it is explicit,
  scoped, and reviewable.

## Current state and known gaps

The current implementation is valuable but intentionally narrower than this
end state:

- the TUI is the primary interactive surface and `U` now enters the shared
  updater execution lane;
- provider discovery covers a broad set of installed managers, language tools,
  known self-updaters, npm prefixes, AI tools, Warp, and declared app metadata,
  but it reports missing, delegated, observed-only, and unsupported sources;
- module manifests, local integrity, registry parsing, store planning, lifecycle
  plans, signatures, process previews, and transaction simulations exist, but
  optional module installation/activation/execution is not production-enabled;
- the seven first-party module directories are source-level packages at
  different maturity levels, not seven active end-user modules;
- uninstall, cleanup, quarantine/restore, arbitrary module execution, and
  production third-party trust remain unavailable;
- Windows runtime proof, macOS race closure, OS capability/network isolation,
  exact recovery completion, native rollback, final-artifact platform matrices,
  and release operations remain open.

Do not “fix” this documentation by changing `planned`, `observed_only`, or
`blocked` labels to make the product appear more complete. The correct next
step is to implement the missing foundation contract and then promote a module
only when its evidence satisfies the contract.

## First-session handoff checklist

Run from the repository root before changing code or contracts:

~~~bash
git status --short --branch
git fetch --prune origin
git log -1 --oneline
git branch -vv
cargo fmt --all -- --check
cargo test --workspace --locked
cargo run --locked -- doctor
cargo run --locked -- scan --dry-run
git diff --check
~~~

Then read the source and tests owning the specific contract. Keep the current
TUI and CLI behavior observable while working. For every implementation change:

1. update the narrow contract and owning module README;
2. update this handoff and `project-status-and-resumption.md` if the user path,
   maturity, validation, or next dependency changes;
3. add or update the acceptance-ledger cells before calling a platform/provider
   supported;
4. run the smallest relevant tests, then the repository validation baseline;
5. inspect the complete task-owned diff and `git diff --check`;
6. commit and push only explicit task-owned files, then record the commit,
   validation, peer-sync results, blockers, and rollback path in the private
   runtime.zero handoff note.

## Non-negotiable invariants

- One canonical evidence, finding, plan, confirmation, transaction, receipt,
  verification, and recovery model.
- One primary TUI workflow and one equally capable scriptable CLI; neither may
  drift into a second authority path.
- Installed, enabled, active, supported, and authorized are separate states.
- Disabled modules do no collection, network, scheduled work, UI action, or
  mutation; disabling does not delete their data.
- A module cannot grant itself capabilities, trust, elevation, network, or
  remote authority through configuration or a manifest.
- Manager-native operations precede direct filesystem mutation; quarantine and
  rollback precede permanent deletion.
- Credentials, sessions, browser profiles, private projects, backups, unknown
  data, and shared components remain blocked until a dedicated contract proves
  ownership and recovery.
- A provider name or executable on PATH is not enough to authorize an action.
- No automatic retry, startup repair, background service, implicit migration,
  surprise install, or silent enablement.
- Every support claim is tied to a named platform/provider/module acceptance
  cell and final-artifact evidence.

The product is successful when adding a new capability means adding a reviewed,
independently manageable module that fits these invariants—not adding another
special case to the core. That is the end game the next shift should preserve.
