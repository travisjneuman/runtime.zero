# Project Status and Resumption Guide

## Snapshot identity

- **Reviewed:** 2026-08-09.
- **Product status:** active pre-alpha development; not production-ready and not
  a supported release.
- **Canonical branch:** `main`.
- **Reviewed repository baseline:** `6f6e5d4177ef9772575c8bb4a3931aaa9a156e2e`.
- **Latest behavior-changing baseline in that snapshot:**
  `1825beb9bf8e02b81f516b4c9dbf3c8cadfeb5f0`.
- **CLI version:** `0.1.0`.
- **Release posture:** blocked; schema-1 release evidence cannot authorize a
  release.
- **Current writes:** explicit user-local store scaffolding and an explicit
  manager-update apply lane exist. Uninstall, cleanup, module installation or
  activation, quarantine/restore, and third-party module execution remain
  unavailable.

This is the public-safe starting point for future work. The previous 2026-07-30
pause handoff remains useful historical evidence, but it was superseded by the
2026-08-01 usability, updater, and native-monitor continuation. Do not resume
from the old pause commit or its test totals without reviewing all later work.

For document precedence and the full topic map, see
[`documentation-index.md`](documentation-index.md).

## Executive assessment

`runtime.zero` is a substantial safety-contract foundation with a useful Mac
read surface and one deliberately explicit updater write path. It is not close
to a defensible 1.0 release yet because most platform runtime cells, six of the
seven domain families, production module lifecycle/trust, rollback, recovery,
packaging channels, accessibility, and release operations remain incomplete.

The strongest implemented areas are:

- bounded, privacy-explicit local inventory;
- path-free installed-software catalog and interactive TUI;
- native read-only system monitoring;
- shared validation, resource, privacy, capability, error, confirmation,
  cancellation, filesystem, transaction, registry, lifecycle, performance, and
  release-ledger contracts;
- fixture and synthetic evidence for trust, staging, quarantine/restore,
  findings, and module transport;
- deterministic local packaging/SBOM/notice generation;
- an explicit manager update lane with fresh discovery, exact confirmation,
  durable local evidence, direct bounded execution, and post-action discovery.

The largest immediate risks are:

- updater execution is production-shaped but does not yet consume every
  foundation production primitive it documents;
- executable allowlisting is path-based in the core updater lane and is not yet
  bound to the opened-artifact lease;
- the Unix process group is containment, not a sandbox, and Windows production
  process execution fails closed;
- updater journal/receipt publication is not yet the full canonical
  registry-last commit-coordinator flow, and native rollback is absent;
- no real package update has been accepted as production evidence;
- public and private documentation had drifted around the old pause state,
  validation totals, website/TUI maturity, and write-capable commands.

## Current user-visible product

### Launch and routing

- Bare `rz0` opens the full-screen TUI only when stdin and stdout are terminals
  and recognized automation variables are absent.
- `rz0 --tui` explicitly requires that interactive environment and fails with a
  usage error rather than silently falling back.
- `rz0 --no-tui`, `rz0 --json`, explicit subcommands, pipes, redirects, and
  automation remain scriptable.
- The TUI uses Crossterm for terminal lifecycle and Ratatui for widgets. Its
  guard restores raw mode, cursor visibility, mouse capture, and the alternate
  screen on normal exit and panic unwinding.
- Scriptable output treats a normal broken pipe as a clean exit.

### Current command surface

```text
rz0
rz0 --tui
rz0 --no-tui
rz0 --json
rz0 --color auto|always|never
rz0 --version
rz0 doctor [--format json]
rz0 apps [--format text|json]
rz0 uninstall plan <installed-software-id> [--format text|json]
rz0 scan --dry-run [--include-raw-paths] [--format text|json]
rz0 monitor [--format text|json]
rz0 updates --dry-run --fixture <evidence.json> [--plan] [--queue] [--format text|json]
rz0 updates --dry-run --manager <id> --manager-output <path> --executable <path> [--plan] [--queue] [--format text|json]
rz0 updates --dry-run --probe --manager <id> --executable <path> --allow-network-read [--plan] [--queue] [--format text|json]
rz0 updates --apply --probe --manager <id> --executable <path> --allow-network-read --allow-network-write (--action <id> | --all) [--accept-no-rollback] [--challenge-issued-unix-seconds <seconds>] [--confirm <phrase>] [--format text|json]
rz0 modules [--from <directory>] [--format text|json]
rz0 modules validate <manifest.json> [--format text|json]
rz0 modules install --dry-run <package> [--format text|json]
rz0 store plan [--format json]
rz0 store status [--store-root <path>] [--format json]
rz0 store init --dry-run [--format json]
rz0 store init --yes [--format json]
```

Run `rz0 --help` and the subcommand help at the reviewed source revision for the
exact parser contract. `--json` is accepted by several commands as an alias even
where compact examples use `--format json`.

### Capability and write matrix

| Surface | Reads | Network | Writes | Current status |
| --- | --- | --- | --- | --- |
| `doctor` | Built-in posture/platform class | No | No | Implemented |
| `apps` | Bounded local inventory | No | No | Implemented; path-free catalog |
| `scan --dry-run` | Bounded local inventory | No | No | Implemented; paths redacted by default |
| `monitor` | Native local counters | No | No | Implemented; metric depth varies by platform |
| TUI startup/`r` | Cached or refreshed local inventory | No | No | Implemented |
| TUI `u` / updater `--probe` | Direct manager availability query | May read remote metadata | No product write | Implemented for bounded probes; manager/runtime proof incomplete |
| updater fixture/captured output | Caller-selected local evidence | No | No | Implemented |
| `updates --apply` | Fresh manager evidence and verification | Explicitly acknowledged; not OS-isolated | Manager plus runtime.zero journal/receipt writes | Implemented pre-alpha lane; not production-supported |
| module validation/install planning | Local manifest/package bytes | No | No | Implemented; planning only |
| store plan/status | Local state metadata | No | No | Implemented |
| `store init --yes` | Existing store state | No | User-local runtime.zero scaffolding | Implemented on Unix; Windows fails closed |
| uninstall review | Local catalog | No | No | Implemented review only |
| uninstall/cleanup/module lifecycle execution | — | — | — | Not implemented |

The `--allow-network-read` and `--allow-network-write` flags are explicit intent
and policy acknowledgements. They do not yet create an operating-system network
sandbox around a manager process.

## Interactive TUI

The interactive dashboard currently has six sections:

1. **overview** — inventory, identity-group, update-check, and uninstall-review
   counts;
2. **local store** — store initialization, registry, and receipt posture;
3. **installed software** — one canonical software list with per-item details;
4. **modules** — installed versus planned first-party module posture;
5. **actions** — available CLI actions and required gates;
6. **system monitor** — live native resource and bounded process rows.

Current controls include:

- `r` refreshes bounded inventory while preserving view context;
- `u` performs an explicit manager availability check and may read network
  metadata without applying an update;
- `m` jumps to the monitor, which refreshes once per second;
- `/` searches the cached catalog; `f` cycles filters and `s` cycles sort;
- arrows and `j`/`k` move selection;
- Home/End jump to region boundaries;
- Tab/Shift+Tab cycle navigation, details, and command focus;
- Enter/Space toggle details;
- the mouse wheel advances the targeted list by three rows;
- Esc backs out through search/details/help/focus before quitting;
- `h` or `?` toggles help; `q` quits.

The TUI does not directly execute an update or uninstall. It shows exact CLI
entry points where an action exists. Keep one software list; do not recreate
parallel update/uninstall lists that duplicate the same objects.

## Inventory and software identity

The installed core embeds the `modules/inventory` library as a bounded,
read-only foundation adapter. The separate `rz0-inventory` development binary
and planned module manifest are not installed or executed by core.

Current built-in sources include:

- process PATH and allowlisted direct executable discovery;
- persisted Windows User/Machine PATH and standard uninstall registry views;
- direct macOS `.app` bundles under known roots, bounded `Info.plist` versions,
  and Homebrew Cellar/Caskroom directory metadata;
- bounded Linux XDG desktop entries;
- optional exact-path version probes on Unix through the shared bounded process
  host; Windows probes fail closed.

Privacy posture:

- `rz0 apps` omits paths;
- `rz0 scan --dry-run` redacts paths by default;
- `--include-raw-paths` is local-only and makes the result unsuitable for the
  support-export privacy gate;
- dashboard JSON omits software names through a private summary model;
- application names, package names, versions, and publishers can still be
  sensitive and require review before sharing.

Software identity groups are now deterministic and preserve source records and
version disagreement. They remain heuristic when records are joined primarily
by normalized display name. They are useful UI provenance, not permanent global
product IDs or mutation authority.

Coverage remains incomplete. Apple package receipts, MacPorts/Nix/language
managers, services, launch agents/daemons, drivers, browser extensions,
persistence, containers, and many Linux/Windows package sources are not yet in
the installed catalog.

## Updater implementation boundary

### Discovery and planning

The updater can consume:

- a strict local finding fixture;
- bounded captured manager output;
- one explicit live manager probe.

Homebrew JSON plus APT, DNF, Pacman, and MacPorts text parser slices exist.
Winget, Zypper, Snap, and Flatpak have probe specifications but currently fail
closed because their output parser is not yet accepted as locale-safe. Probe
support is not equivalent to runtime support on the full OS matrix.

Updater records become shared finding reports, finding-bound action plans, and
serial queue plans. Those artifacts remain dry-run evidence with
`writes_attempted: false`; the core apply lane separately selects one exact
planned action and performs its own confirmation/execution transition.

### Explicit apply lane

`updates --apply` requires:

1. an explicit live `--probe` and supported manager/platform pair;
2. an allowlisted absolute manager path;
3. explicit network-read and network-write acknowledgement;
4. exactly one action ID or an interactive serial `--all` flow;
5. manual-recovery acknowledgement when rollback is unproven;
6. an initialized private runtime.zero state root;
7. a newly generated five-minute exact phrase bound to a single-action plan;
8. durable single-use confirmation evidence;
9. a direct, environment-cleared, no-shell manager process with bounded output
   and timeout;
10. fresh manager evidence showing that the exact candidate is no longer
    available.

The lane never invokes `sudo` or an interactive privilege helper. Unix actions
that require elevation require the existing process to be root. Windows
production execution fails closed because inherited-handle and race-free process
containment are incomplete.

### Updater hardening still required

The lane is intentionally pre-alpha and is not release evidence:

- the allowlisted manager path is not yet pinned through
  `crates/artifact-identity` into the actual core spawn;
- Unix process-group teardown does not prevent a hostile child from creating a
  new session and is not a filesystem, syscall, privilege, or network sandbox;
- capability and network flags are validated policy, not OS-enforced denial;
- cancellation is not propagated through every discovery/process/journal/
  receipt boundary;
- manager-native rollback and automated safe recovery do not exist;
- the updater-specific journal and receipt do not yet use the complete canonical
  commit-receipt/registry-last coordinator, and receipt publication after a
  successful manager command still needs interruption-proof reconciliation;
- real update, failure, drift, power-loss, and rollback evidence is missing on
  disposable Windows/macOS/Linux hosts;
- no current public release supports this lane.

Treat `product_execution_authorized: true` in an updater execution report as a
record of that one locally confirmed lane invocation, not as production, module,
or release authorization.

## Uninstall and cleanup boundary

`rz0 uninstall plan <id>` emits a path-free `uninstall_review`. It is a UX
review, not a shared executable action plan. It has no write set and cannot be
confirmed into execution.

| Evidence | Current posture |
| --- | --- |
| Protected system application | Details only; blocked |
| Homebrew formula/cask | Manager-owned uninstall review |
| Local/user application bundle | Quarantine-first review |
| Unknown ownership/source | Details only; unsupported |

Before uninstall can execute, exact catalog evidence must become a shared
finding and action plan; manager or bundle identity must be race-resistant; the
confirmation, transaction, receipt, rollback/quarantine, cancellation, and
fresh re-inventory paths must all have real platform proof. Direct recursive
application deletion remains prohibited.

## Module catalog and lifecycle

The frozen 1.0 catalog contains seven equal-priority families:

1. inventory/environment;
2. updater;
3. uninstall;
4. leftovers;
5. cache management;
6. security/integrity;
7. report/export.

All seven manifests remain `planned`. Current maturity:

| Family | Current implementation | Major missing work |
| --- | --- | --- |
| Inventory/environment | Built-in collector library and separate development binary | Full source/platform parity and signed lifecycle integration |
| Updater | Synthetic/live evidence parsing, finding/plan/queue, and core explicit apply lane | Production process identity/isolation, rollback/recovery, managers, and complete runtime matrix |
| Uninstall | Synthetic classifier and live Mac review UX | Live finding/action binding and safe execution on every platform |
| Leftovers | Synthetic exact-runtime-owned classifier | Live bounded ownership discovery and quarantine execution |
| Cache management | Synthetic ownership-aware classifier | Live adapters, budgets, quarantine/restore, and manager policy |
| Security/integrity | Synthetic digest observation classifier | Trusted baselines, live reads, incident semantics, and platform proof |
| Report/export | Strict summary-only development binary | Core lifecycle/protocol integration and final-artifact runtime evidence |

The core can validate manifests and package hashes and can plan installation.
It cannot install, activate, invoke, repair, migrate, upgrade, deactivate, or
uninstall a module. Detached signatures use public test keys only. Production
keys, provenance, revocation, capability enforcement, and third-party trust do
not exist.

## Foundation ownership map

| Package | Implemented responsibility | Important open boundary |
| --- | --- | --- |
| `validation-contract` | Canonical lexical grammar | Lexical validity never grants authority |
| `resource-contract` | Shared ceilings | Target-specific measurement and enforcement remain |
| `capability-contract` | Vocabulary and exact schema subsets | No OS capability broker |
| `error-contract` | Stable codes and privacy/retry classification | Broad adapter integration and localization remain |
| `configuration-contract` | Immutable default-deny schema 1 | No reviewed user configuration/migration model |
| `privacy-contract` | Report-local redaction | Not anonymization; broader adapters need classification |
| `diagnostics-contract` | Private config-bound doctor report | Support/repair workflows remain |
| `inventory-contract` | Strict bounded evidence | Coverage and runtime parity remain |
| `finding-contract` | Shared path-free classification | Live output exists for updater only; other domains remain synthetic/review-only |
| `action-plan` | Finding-bound dry-run plans | General production execution authority remains separate |
| `confirmation-contract` | Exact short-lived single-use confirmation | Not authority by itself |
| `cancellation-contract` | First-writer-wins cancellation/deadline | Remaining process/write integration |
| `process-host` | Bounded direct Unix transport and test containment | Identity binding, sandboxing, Windows production host |
| `secure-fs` | Opened-directory state I/O | Windows ACL creation/runtime proof and broad filesystem matrix |
| `artifact-identity` | Same-open-handle digest/identity and partial spawn leases | Core updater/module host integration; exact macOS primitive |
| `module-trust` | Public-test-key signature and staging contracts | Production trust root, signer, provenance, revocation |
| `module-protocol` | Unauthorized preview and guarded test helper | Production module host and capability broker |
| `module-lifecycle` | Eight digest-bound planning transitions | No lifecycle execution |
| `registry-contract` | Canonical installed state | No module install publication path |
| `transaction-contract` | Hash chain, durable evidence, registry-last coordinator | Domain execution/rollback and platform power-loss proof |
| `performance-contract` | Six command budgets | Current catalog/TUI/update operations and all targets |
| `release-contract` | Bounded target × module × stage ledger | RC target freeze and real evidence population |

## Validation baseline

The 2026-08-09 documentation review reproduced the current default toolchain
baseline with Rust/Cargo 1.96.0:

- `cargo fmt --all -- --check` passed;
- `cargo test --workspace --locked` passed **332** tests across 96 test/doc-test
  suites;
- `cargo test --workspace --locked --all-features` passed **343** tests across
  96 suites;
- strict native Clippy passed over all workspace targets/features with warnings
  denied;
- live `doctor`, redacted `scan`, path-free `apps`, and native `monitor` JSON
  contracts parsed and retained `read_only: true` / `writes_attempted: false`;
- 31 local workspace packages and 119 external packages resolved from 150 lock
  entries;
- all repository-relative Markdown links resolved.

`cargo-deny` and `cargo-audit` were not installed on the review host, so the
2026-07-29 advisory/license evidence was not refreshed. That older audit remains
valid only for its named lockfile/source snapshot. Cross-target compilation,
final-artifact packaging, PTY, performance, DMG, Windows/Linux runtime, and
real update execution were not rerun as part of the documentation-only review.

## Known product limitations and design debts

### Product and UX

- No stable 1.0 UX or compatibility guarantee exists.
- The dashboard uses one cached inventory snapshot until `r`; update checks have
  their own explicit refresh.
- Software identity grouping can create heuristic/name-normalized relationships;
  it preserves disagreement but is not a canonical package identity service.
- TUI update writes are CLI handoffs rather than in-dashboard confirmation.
- Uninstall, cleanup, integrity remediation, report export, and module lifecycle
  are not end-to-end product flows.
- Help, manual pages, shell completions, localization policy, recovery UX, and
  screen-reader evidence are incomplete.
- The public website mock predates the real six-section TUI and needs a
  separately approved parity/deployment pass.

### Platform behavior

- Windows inventory/monitor/process/filesystem behavior lacks full real-runtime
  evidence across the declared client/server matrix.
- Linux app/monitor behavior lacks the distro, manager, terminal, sandbox, and
  packaging matrix.
- macOS evidence is concentrated on a current Apple Silicon host; Intel hardware
  and older macOS remain unproven.
- Monitor depth differs: first-sample CPU can be unavailable; macOS process CPU
  rows are not currently sampled; Windows interface byte counters and running
  process counts are deferred; restricted containers can hide host metrics.
- No platform has complete power-loss, locked-file, low-space, cross-filesystem,
  ACL/ownership, privilege, and recovery evidence.

### Release and operations

- No production release, installer, package-channel, support promise, or
  security-supported version exists.
- No production signing key or key-compromise/revocation workflow exists.
- Existing local ZIP/DMG evidence is unpublished, unsigned, and tied to earlier
  commits; it must be rebuilt for an RC.
- No approved CI/release workflow, beta/RC operation, incident runbook, support
  process, or compatibility lab is complete.

## Dependency-ordered continuation plan

1. **Keep documentation and contracts aligned.** Start from this guide,
   `roadmap.md`, and `production-readiness.md`; do not resume from the old pause
   checklist.
2. **Harden the updater exception before expanding writes.** Bind opened
   executable identity to spawn, unify updater durable publication with the
   canonical transaction/receipt model, propagate cancellation, add recovery,
   and prove real disposable-host failure/rollback behavior.
3. **Complete software identity and inventory parity.** Replace heuristic-only
   joins with source-specific durable provenance where available and implement
   missing package/service/persistence sources across all three platform
   families.
4. **Complete process/filesystem/capability foundations.** Resolve macOS exact
   spawn, Windows inherited handles and suspended Job assignment, Linux/macOS
   sandbox policy, network/elevation enforcement, Windows ACL creation, and
   platform filesystem matrices.
5. **Bind uninstall reviews to the shared action pipeline.** Keep manager-native
   and quarantine-first methods, with protected/unknown data blocked.
6. **Advance every module family equally.** Add synthetic/adversarial fixtures,
   live adapters, CLI/JSON/TUI, lifecycle integration, and platform runtime
   evidence without duplicating foundation policy.
7. **Populate the frozen acceptance ledger.** Freeze exact RC targets/managers/
   channels and fill every target × seven-module × 12-stage cell with reviewed
   evidence or evidence-backed not-applicable status.
8. **Finish UX, packaging, security, and operations.** Accessibility, terminals,
   installers/packages, reproducibility, legal review, vulnerability response,
   support, beta/RC, website parity, and honest release instructions are all
   release-blocking.
9. **Review external actions separately.** Publication, package submission,
   website deployment, signing credentials, workflows, recurring automation,
   paid services, third-party execution, and non-disposable-host mutation remain
   explicit approval events.

## First-session command checklist

```bash
git status --short --branch
git fetch --prune origin
git pull --ff-only origin main
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --workspace --locked --all-features
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo run --locked -- apps --format json
cargo run --locked -- scan --dry-run --format json
cargo run --locked -- monitor --format json
cargo run --locked -- doctor --format json
git diff --check
```

Run dependency, cross-target, packaging, final-artifact, PTY, performance,
accessibility, and disposable-host lanes when relevant. A release claim requires
the complete frozen matrix, not only this smoke list.

## Invariants to preserve

- Report first, dry-run first, quarantine first, exact confirmation first.
- One canonical software list with per-object options.
- No credentials, sessions, private keys, host identity, or private paths in
  public evidence.
- No direct recursive deletion of applications or unknown data.
- Manager-native actions before direct filesystem cleanup.
- No module execution until trust, identity, capability, isolation, lifecycle,
  transaction, and runtime gates pass.
- No weaker path-based Windows mutation fallback.
- No automatic retry in schema 1.
- Evidence, plans, signatures, leases, confirmations, receipts, diagnostics,
  and ledgers never authorize broader mutation or release by themselves.
- Windows, macOS, Linux, and all seven frozen module families remain equal
  release requirements unless the 1.0 scope is explicitly changed before RC.
