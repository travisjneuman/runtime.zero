# Project Status and Resumption Guide

## Snapshot identity

- **Reviewed:** 2026-08-20.
- **Product status:** active pre-alpha development; not production-ready and not
  a supported release.
- **Canonical branch:** `main`.
- **Reviewed starting baseline:**
  `e015b17` (`Redesign TUI and add Rust toolchain surface`).
- **Current behavior implementation:**
  `e015b17` plus documentation-only follow-up on `main`.
- **CLI version:** `0.1.0`.
- **Release posture:** blocked; schema-1 release evidence cannot authorize a
  release.
- **Current writes:** explicit user-local Unix store scaffolding and a working
  macOS/Linux manager-update executor exist. Uninstall, cleanup,
  quarantine/restore, module lifecycle execution, Windows execution, and
  third-party execution remain unavailable.

For the product end state, module contract, enable/disable semantics, delivery
waves, and next-shift checklist, see
[`engineering-handoff.md`](engineering-handoff.md). For document precedence and
the full topic map, see [`documentation-index.md`](documentation-index.md). The
2026-07-30 pause handoff and earlier plans remain historical evidence only.

## Executive assessment

`runtime.zero` now has a broad provider-driven product surface and a working
pre-alpha updater executor. It remains far from a defensible 1.0 release because
full platform source parity, OS capability isolation, rollback, exact recovery
completion, module trust/lifecycle, uninstall/cleanup execution, accessibility,
compatibility labs, packaging channels, and release operations are incomplete.

The product direction beyond the initial seven release-gated families is a full
system-management platform composed of independently installable and
enableable modules. Users should be able to choose inventory, updates,
developer/AI tools, services, cleanup, security, network, hardware, backup,
automation, and other reviewed capabilities without forcing every feature into
the core or enabling every module. That is the end-state direction; the current
repository has the contracts and planning model but not the executable optional
module lifecycle. See [`engineering-handoff.md`](engineering-handoff.md) for the
target state and sequencing.

The strongest implemented areas are:

- bounded, privacy-explicit software/package/service inventory with explicit
  source status;
- source-specific software identifiers and deterministic identity grouping that
  preserves disagreement;
- path-free installed-software catalog, task-first five-workspace TUI, and native monitor;
- privacy-reviewed `rz0 report` summaries with no automatic sharing;
- shared validation, resource, privacy, capability, error, finding/action,
  confirmation, cancellation, filesystem, artifact-identity, transaction,
  registry, lifecycle, performance, and release-ledger contracts;
- dry-run uninstall findings and sealed manager action plans without execution;
- Linux opened-executable identity-to-spawn binding, bounded process-group
  teardown, caller cancellation, exact updater write evidence, canonical
  external-effect receipts, and read-only recovery assessment;
- provider-native macOS/Linux updater execution for Homebrew formulae/casks,
  Apple Software Update, npm prefixes, pip, RubyGems, rustup, uv, AIUP, Cargo,
  Warp, known self-updaters, and declared Electron/Squirrel application
  channels, with explicit delegated, missing, and observed-only source states;
- deterministic local packaging/SBOM/notice generation, shell completions, a
  manual page, and operator guides.

The largest immediate risks are:

- macOS uses a last-moment direct-path identity/digest binding because Darwin
  exposes no public fexecve-style primitive; this is weaker than Linux's held
  descriptor launch and remains pre-alpha;
- Windows updater execution fails closed because exact process-image binding and
  race-free Job Object containment are incomplete;
- Unix process groups are containment aids, not syscall/filesystem/network/
  privilege sandboxes, and a hostile child may attempt session escape;
- cancellation is integrated into confirmed execution but not every discovery,
  verification, and write boundary;
- a valid external-effect receipt can identify an interrupted final journal
  commit, but no exact approval-bound recovery-completion command exists;
- native rollback, exact recovery completion, and disposable-host power-loss/
  fault proof remain absent; elevated managers use non-interactive `/usr/bin/sudo`;
- inventory service records are metadata presence/configuration evidence, not
  complete live-status, ownership, dependency, or actionability proof.

### 2026-08-17 live updater evidence

The current development Mac produced a bounded live review of 20 provider
sources and 85 planned actions. Native apply support is present for every
source that returned an exact manager/update adapter in that review, including
Homebrew formulae/casks, Apple Software Update, both discovered npm prefixes,
pip, RubyGems, AIUP-managed tools, crates.io Cargo installs, Warp's standalone
CLI store, and declared Electron/Squirrel releases. Deno is explicitly
delegated to its Homebrew formula because the installed binary lacks native
self-upgrade support. MacPorts, Mac App Store, and Hermes were reported as
missing on this host; 12 Sparkle bundles were observed-only because Sparkle's
public tooling does not provide a generic external app-update command.

Live smoke work committed OMP, Pi/npm-prefix, and AIUP effects through the
canonical receipt path. Warp's standalone CLI store switched to and verified
the signed current version, but earlier live transactions reached recovery
status before receipt publication because the receipt contract rejected a
valid large executable and native binding suffix; the contract and regression
test are now corrected. T3 Code's Electron/Squirrel action is executable and
has a current release target, but the running T3 process was left open; quit
the app normally before applying that action from a fresh plan.

## Current command surface

```text
rz0 [--tui|--no-tui|--json] [--color auto|always|never]
rz0 --version
rz0 doctor [--format text|json]
rz0 apps [--format text|json]
rz0 report [--format text|json]
rz0 uninstall plan <installed-software-id> [--executable <absolute-path>] [--format text|json]
rz0 scan --dry-run [--include-raw-paths] [--format text|json]
rz0 monitor [--format text|json]
rz0 toolchain [--format text|json]
rz0 completions <bash|zsh|fish|powershell>
rz0 updates --dry-run --fixture <evidence.json> [--plan] [--queue] [--format text|json]
rz0 updates --dry-run --manager <id> --manager-output <path> --executable <path> [--plan] [--queue] [--format text|json]
rz0 updates --dry-run --probe --manager <id> --executable <path> --allow-network-read [--plan] [--queue] [--format text|json]
rz0 updates --dry-run --all-providers --allow-network-read [--plan] [--queue] [--format text|json]
rz0 updates --recovery-status --transaction <id> [--format text|json]
rz0 updates --apply --probe --manager <id> --executable <path> --allow-network-read --allow-network-write (--action <id> | --all) [--accept-no-rollback] [--challenge-issued-unix-seconds <seconds>] [--confirm <phrase>] [--format text|json]
rz0 updates --apply --all-providers --allow-network-read --allow-network-write [--accept-no-rollback] [--format text]
rz0 updates --apply --all-providers --allow-network-read --allow-network-write --action <id> [--accept-no-rollback] [--challenge-issued-unix-seconds <seconds>] [--confirm <phrase>] [--format text|json]
rz0 modules [--from <directory>] [--format text|json]
rz0 modules validate <manifest.json> [--format text|json]
rz0 modules install --dry-run <package> [--format text|json]
rz0 modules lifecycle-plan <operation> --dry-run --module-id <id> --from-state <state> --to-state <state> [--from-version <version>] [--to-version <version>] [--format text|json]
rz0 store plan [--format text|json]
rz0 store status [--store-root <path>] [--format text|json]
rz0 store init --dry-run|--yes [--format text|json]
```

`rz0 --help` and subcommand help are the exact parser contract. Static
completion source has parser-coverage tests but is not generated by the parser;
[`docs/man/rz0.1`](man/rz0.1) is manually reviewed and must remain synchronized.

## Capability and write matrix

| Surface | Network | Writes | Current status |
| --- | --- | --- | --- |
| `doctor` | No | No | Implemented privacy-safe posture report |
| `apps` | No | No | Implemented path-free catalog; names/IDs remain sensitive |
| `scan --dry-run` | No | No | Implemented; paths redacted by default |
| `monitor` | No | No | Implemented one-shot native snapshot; depth varies |
| `report` | No | No | Implemented privacy-reviewed summary; external sharing never auto-authorized |
| TUI startup/`r` | No | No | Implemented inventory/monitor refresh |
| TUI `u` / updater `--probe` | Manager may read remote metadata after acknowledgement | No product write | Bounded provider review/probe |
| TUI `U` selected update | Provider metadata plus manager network write where required | Manager plus private journal/receipt writes | Direct shared macOS/Linux update flow with exact TUI confirmation; Windows blocked |
| `updates --all-providers` | Providers may read remote metadata after acknowledgement | No product write | Provider-driven bounded review across installed managers, language environments, self-updaters, and declared app metadata; missing, observed-only, and unsupported sources remain warnings |
| updater fixture/captured output | No | No | Implemented review/planning |
| `updates --recovery-status` | No | No | Implemented deterministic evidence assessment only |
| `updates --apply` | Explicit read/write acknowledgement; not OS-isolated | Manager plus private journal/receipt writes | Working macOS/Linux pre-alpha lane with receipts; Windows blocked |
| uninstall plan | No | No | Shared finding and optional sealed action plan; no execution |
| module validation/install planning | No | No | Implemented planning only |
| store plan/status | No | No | Implemented read-only inspection |
| `store init --yes` | No | Runtime.zero-owned user-local scaffold | Unix only; Windows blocked |
| uninstall/cleanup/module lifecycle execution | — | — | Not implemented |

Network flags express intent; they do not create an OS network sandbox. No
command uploads a report or installs an elevation helper.

## Interactive TUI

Bare `rz0` opens the full-screen TUI only for an interactive stdin/stdout pair
without recognized automation variables. Explicit subcommands never enter the
TUI. Terminal guards restore raw mode, cursor, mouse capture, and alternate
screen on normal exit and panic unwinding; ordinary broken pipes are clean exits
for scriptable output.

The five workspaces are Home, Toolchain, Software, System, and Diagnostics.
The TUI renders a loading shell before the full inventory/monitor worker
finishes, and `r` is the only explicit retry. Controls include `r` inventory
refresh, `u` explicit provider availability, visible `Review action [U]`, `m`
System, `/` search, `f` filter, `s` sort, arrows/`j`/`k`, Home/End,
Tab/Shift+Tab, Enter/Space, mouse wheel, `h`/`?`, Esc, and `q`.

The TUI has no second mutation implementation: `U` enters the same exact updater
plan, confirmation, identity-bound process, receipt, and verification path as
the CLI. Uninstall, cleanup, and recovery completion remain outside the direct
TUI action set.

## Inventory and identity

The installed core embeds the bounded `modules/inventory` library. Current
metadata sources include:

- process PATH and allowlisted executable discovery;
- persisted Windows User/Machine PATH, standard uninstall registry views, and
  service/driver registry metadata;
- macOS application bundles and bundle IDs, Homebrew Cellar/Caskroom roots,
  MacPorts roots, Apple Installer receipt plists, and launchd plist labels;
- Linux XDG desktop entries/desktop IDs, direct dpkg status and pacman local
  metadata, and systemd unit-file labels;
- optional exact-path Unix version probes through the shared process host.

Collectors do not invoke package managers or service controllers for baseline
inventory. Every source retains independent `ok`, `partial`, or `unavailable`
status, bounded duration/warnings, and deterministic records. Path-bearing app,
package, and service locations participate in report-local redaction.

`SoftwareIdentifier { kind, value }` records source-native identity such as a
bundle ID, package ID, desktop ID, package receipt ID, product code, or registry
product key. The catalog groups records sharing an identifier before applying a
name-normalized heuristic. Group confidence and version disagreement remain
visible. IDs improve local reconciliation but are not universal product IDs and
never authorize mutation.

Coverage remains incomplete: RPM/DNF, Snap, Flatpak, AppImage, Nix, language
managers, containers, browser extensions, live service status/dependencies,
MSIX/AppX/Winget/Chocolatey/Scoop, and many persistence/driver details await an
explicit 1.0 scope and target-native proof.

## Support summary and privacy

`rz0 report` collects redacted live inventory and private diagnostics, then
emits only strict support-summary fields and domain-separated digests. It omits
raw reports, paths, host/user identity, app/service names, process output, and
free-form warnings. Text and JSON are deterministic for one input. The result
sets `local_export_ready` only after the privacy gate and always keeps
`external_sharing_authorized: false`.

`apps` is path-free but not support-safe by implication: names, versions,
publishers, and source identifiers may be sensitive. A raw-path scan is local
only. See [`privacy-and-sharing.md`](privacy-and-sharing.md).

## Updater implementation boundary

### Discovery and planning

The updater consumes strict fixtures, bounded captured output, or one explicit
live probe. Homebrew JSON and bounded APT/DNF/Pacman/MacPorts parser slices exist.
The all-provider lane also has native update adapters for Homebrew,
Apple Software Update, npm prefixes, language tools, AIUP-managed tools,
crates.io Cargo installs, Warp's standalone CLI store, and declared
Electron/Squirrel releases. Winget/Zypper/Snap/Flatpak specifications fail
closed where parsers are not accepted. Findings, action plans, and queue plans
remain non-authorizing until the apply lane consumes an exact confirmation.

Live discovery observes the exact manager artifact and seals its SHA-256, size,
and platform identity into each plan. Replacement invalidates plan/confirmation
identity.

### Confirmed execution sequence

The narrow lane requires one exact live plan, network acknowledgements, an
initialized private store, an action-scoped five-minute phrase, single-use
confirmation consumption, and no-rollback acknowledgement when applicable.
It then:

1. obtains a `BoundExecutable` before consuming confirmation;
2. serializes the inheritable-descriptor audit/spawn boundary;
3. on Linux, launches a direct native ELF manager through the held
   `/proc/self/fd/<fd>` identity; script/interpreter chains are blocked;
4. records canonical `prepared`, `apply_started`, exact `write_intent`, and exact
   `write_verified` journal events;
5. uses the bounded cancellable process host with dedicated Unix process group,
   output ceilings, deadline, kill/reap, and post-spawn executable revalidation;
6. performs fresh installed-only manager verification;
7. synchronizes a canonical `external_effect_commit_receipt` bound to the
   transaction, plan/write set, confirmation, executable identity, arguments,
   bounded process outcome, and verification digest;
8. appends final `committed` evidence only after the receipt is durable.

On Unix, the first SIGINT during the confirmed lane becomes typed
`user_requested` cancellation. The host terminates/reaps the process group and
publishes recovery-required evidence where possible. It does not reverse an
external effect already performed.

macOS manager apply uses direct-path identity/digest revalidation immediately
before spawn. Windows remains blocked at production binding and process-tree
containment. Elevated Unix manager actions use non-interactive `/usr/bin/sudo`;
no password or interactive helper is collected. Known self-updaters may replace
their launcher and are verified through the declared transition plus fresh
provider evidence.

### Recovery assessment

`updates --recovery-status` validates the exact journal and receipt from the
private store and selects one conservative action: abort without writes, verify
an uncertain external effect, require a future explicitly approved final journal
completion, take no action for consistent committed evidence, or refuse
inconsistent evidence. It never mutates, repairs, retries, rolls back, or reruns
a manager.

Still required: exact receipt-bound completion, manager rollback/manual recovery
matrices, cancellation through every pre/post-process boundary, OS capability
and network enforcement, Windows/macOS bindings, and disposable-host drift/
crash/reboot/power-loss proof.

## Uninstall and cleanup boundary

`rz0 uninstall plan <id>` now converts a live catalog record into the shared
finding contract instead of a separate temporary review schema. Manager-owned
software can also receive an exact dry-run action plan when the caller supplies
an allowlisted executable whose artifact identity is sealed successfully.
Without that identity the action remains blocked. Protected software remains
blocked; user/local bundles remain quarantine-first report-only; unknown and
receipt-only ownership cannot execute.

No process, file move, quarantine, deletion, elevation, dependent-package
review, verification, rollback, or recovery occurs. A `planned` action still has
`execution_authorized: false` and cannot be consumed by an uninstall executor
because none exists.

## Module catalog and lifecycle

All seven first-party manifests remain `planned`:

| Family | Current implementation | Major missing work |
| --- | --- | --- |
| Inventory/environment | Embedded read-only collector plus development binary | Full source/platform parity and signed lifecycle |
| Updater | Provider-driven plans plus working macOS/Linux core executor | Windows isolation, rollback/recovery, manager/runtime matrix, release proof |
| Uninstall | Shared synthetic/live findings and dry-run manager plans | Every execution/elevation/quarantine/rollback path |
| Leftovers | Synthetic exact-runtime-owned classifier | Live ownership discovery and quarantine |
| Cache | Synthetic ownership-aware classifier | Live bounded adapters, budgets, quarantine/restore |
| Security/integrity | Synthetic digest classifier | Trusted baselines, live reads, incident review |
| Report/export | Strict module binary plus integrated foundation report | Signed lifecycle and final-artifact platform proof |

The core can validate manifests/hashes and plan installation. It cannot install,
activate, invoke, repair, migrate, upgrade, deactivate, or uninstall modules.
Test-key signatures, schemas, fixtures, process tests, and lifecycle plans do not
provide production trust or authority.

The target lifecycle must make those choices user-visible and reversible:
installed, disabled, enabled/active, degraded/blocked, and action-authorized
are distinct states. Disable stops module-owned collection, scheduling, network
work, UI actions, and mutation while preserving state; uninstall is a separate
explicit data-retention and rollback decision. The target CLI/TUI controls are
not current commands and must wait for foundation-owned registry publication,
trust, configuration, receipts, recovery, and module-host execution.

## Foundation ownership map

| Package | Implemented responsibility | Important open boundary |
| --- | --- | --- |
| `validation-contract` | Canonical lexical grammar | Validity never grants authority |
| `resource-contract` | Shared ceilings | Target-specific measurement/enforcement |
| `capability-contract` | Vocabulary/schema partitions | No OS capability broker |
| `error-contract` | Stable privacy/retry codes | Broad adapter/localization integration |
| `configuration-contract` | Immutable default-deny schema 1 | No user config/migration model |
| `privacy-contract` | Report-local redaction | Not anonymization |
| `diagnostics-contract` | Private config-bound doctor | Production repair/support flow |
| `inventory-contract` | Strict evidence, identifiers, services | Source/runtime parity |
| `support-contract` | Privacy-reviewed summary | External sharing stays human-controlled |
| `finding-contract` | Path-free classification | Findings cannot authorize |
| `action-plan` | Finding-bound dry-run plans | Domain executors remain separate |
| `confirmation-contract` | Exact short-lived single-use confirmation | Not authority alone |
| `cancellation-contract` | First-reason cancellation/deadline | Remaining boundary integration |
| `process-host` | Bounded direct transport and Unix groups | OS sandbox/Windows production host |
| `secure-fs` | Opened-directory state I/O | Windows ACL creation/runtime and FS matrix |
| `artifact-identity` | Same-handle identity plus Linux lease and macOS path-revalidation binding | Windows production binding and cross-platform runtime proof |
| `module-trust` | Test-key signature/staging contracts | Production roots/provenance/revocation |
| `module-protocol` | Unauthorized preview/test child | Production module host |
| `module-lifecycle` | Eight planning transitions | No lifecycle execution |
| `registry-contract` | Canonical installed state | No module install publication |
| `transaction-contract` | Journal, external receipts, coordinator, recovery | Exact domain rollback/platform proof |
| `performance-contract` | Nine read-only command budgets | TUI timing and target-native evidence |
| `release-contract` | Target × module × stage ledger | RC freeze/evidence population |

## Validation baseline

Final validation for
`c2a15c646e9a255f4ac8a6bac48445c015beec30` used Rust 1.96.0
(`rustc ac68faa20`, `cargo 30a34c682`) on `aarch64-apple-darwin`:

- `cargo fmt --all -- --check` passed;
- default workspace tests passed: 359 tests across 97 test/doc-test suites;
- all-feature workspace tests passed: 370 tests across 97 suites;
- strict locked all-target/all-feature Clippy passed with `-D warnings`;
- locked all-target/all-feature compile checks passed for
  `x86_64-unknown-linux-gnu` and `x86_64-pc-windows-msvc`; these are compile
  evidence, not target-native runtime proof;
- an exact-commit release binary passed native doctor, scan, apps, monitor,
  report, dashboard, fixture update queue, and update/recovery-help smokes;
  live redacted inventory observed 273 software and 898 service/persistence
  records on this host;
- the same binary passed all four bounded PTY cases and schema-2 performance
  evidence for all nine operations; both records remained non-authorizing;
- local unsigned portable packaging, artifact-manifest digest verification,
  target-filtered SPDX generation, and third-party notices passed. The temporary
  ZIP SHA-256 was
  `93b7ca37b828b67bb4a88174af069a6f812c3f7ac9ef4be621ce6cc6b7fbaaec`;
- four Python script tests and Python syntax checks passed;
- completion source matched CLI output; Bash and Zsh syntax, PowerShell parsing,
  `mandoc`, relative links across 70 Markdown files, seven module manifests,
  unsafe-DOM patterns, secret/private-path patterns, and diff hygiene passed. Fish and ShellCheck
  were unavailable, so Fish has static coverage rather than a native parser run.

`cargo-audit` and `cargo-deny` were unavailable and were not auto-installed.
Locked metadata still resolved 150 packages (31 workspace and 119 external), and
native target-filtered release metadata covered 119 reachable packages (96
external). The 2026-08-17 continuation additionally passed `cargo fmt --check`,
`cargo test --workspace`, `cargo check --workspace`, `cargo run -- doctor`, and
`cargo run -- scan --dry-run`; it also completed live updater smoke effects on
the development Mac as described above. No uninstall, Cloudflare/site
mutation, release publication, or production release action was run.

The updater/TUI continuation was then committed as
`3f125c5c3d031a67e3f229c2026f066abd8b70dd`; this documentation continuation
starts from that behavior snapshot. The docs-only commit must rerun the
repository checks before it becomes the current handoff baseline.

## Known limitations

### Product and UX

- No stable 1.0 CLI/JSON/API compatibility guarantee exists.
- TUI update actions now enter the shared direct confirmation/execution flow;
  uninstall and cleanup actions remain reviews rather than execution flows.
- No uninstall, cleanup, restore, integrity remediation, or module lifecycle
  execution exists.
- Interactive TUI cross-platform first-frame/refresh review, localization policy,
  migration/repair guides, and human screen-reader review remain incomplete.
- The public website mock predates the real five-workspace product; source/deploy
  changes require separate approval.

### Platforms

- Windows read paths compile but lack the declared real client/server runtime
  matrix; updater/store mutation remains blocked.
- Linux needs native distro/manager/systemd/sandbox/filesystem/package proof.
- macOS evidence remains concentrated on a current Apple Silicon host; exact
  manager spawn, Intel hardware, and older releases are unproven.
- Service records do not yet assert authoritative loaded/running state.
- No platform has complete power-loss, locked-file, low-space, ACL/ownership,
  cross-filesystem, privilege, rollback, and recovery proof.

### Release and operations

- No production release, installer/package channel, supported version, or
  support promise exists.
- No production key, compromise/revocation workflow, approved release pipeline,
  compatibility lab, beta/RC process, incident runbook, or independent security
  review is complete.
- Local ZIP/DMG evidence is unpublished and must be rebuilt from an exact RC.

## Dependency-ordered continuation

1. Freeze 1.0 journeys, targets, managers, schemas, budgets, and acceptance IDs.
2. Finish updater exception hardening: macOS/Windows spawn binding, OS
   capabilities/network/elevation, full cancellation, exact recovery completion,
   manager rollback/manual recovery, and disposable-host fault proof.
3. Complete process/filesystem foundations and target-native containment/ACL/
   mount/filesystem matrices.
4. Finish in-scope package/service/persistence sources and adversarial identity
   reconciliation/runtime privacy tests.
5. Build uninstall execution through manager-native or quarantine-first exact
   plans, with dependency review, confirmation, durable receipts, rollback, and
   recovery.
6. Advance all module families through trust, signed immutable lifecycle,
   least-privilege process hosting, CLI/JSON/TUI, and equal-platform proof.
7. Populate every frozen release-ledger cell with final-artifact evidence or a
   reviewed evidence-backed not-applicable result.
8. Complete accessibility, performance, packaging/install/update/uninstall,
   legal/security/privacy, vulnerability/support/incident, beta/RC, and release
   governance work.
9. Request separate approval before website/deployment, workflows, package
   publication, signing credentials, paid/quota services, or production writes.

## First-session checklist

```bash
git status --short --branch
git fetch --prune origin
git pull --ff-only origin main
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --workspace --locked --all-features
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo run --locked -- doctor --format json
cargo run --locked -- apps --format json
cargo run --locked -- scan --dry-run --format json
cargo run --locked -- monitor --format json
cargo run --locked -- report --format json
git diff --check
```

## Invariants

- Report first, dry-run first, quarantine first, exact confirmation first.
- One canonical software list with source evidence and per-object options.
- No credentials, sessions, private keys, host identity, or private paths in
  public evidence.
- No direct recursive deletion of applications or unknown data.
- Manager-native action before direct filesystem cleanup.
- No module execution before trust, identity, capability, isolation, lifecycle,
  transaction, and runtime gates pass.
- No weaker pathname/Windows mutation fallback.
- No automatic retry in schema 1.
- Evidence, plans, signatures, leases, confirmations, receipts, reports, and
  ledgers never authorize broader mutation or release by themselves.
- Windows, macOS, Linux, and all seven frozen module families remain equal 1.0
  requirements unless the scope is explicitly changed before RC.
