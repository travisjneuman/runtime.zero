# Project Status and Resumption Guide

## Snapshot identity

- **Work status:** current usability continuation completed on 2026-08-01;
  production work remains gated.
- **Source status:** pre-alpha; not a production or 1.0 release.
- **Canonical branch:** `main`.
- **Verified product implementation commit:** `c9a43c59b2e39005c7d74d9087bb6671a8798752`.
- **Previous synchronized commit:** `3a86d2b5709a2100f94fdc1a528c07ad8ef501d1`.
- **Paused baseline commit:** `53d1e3de4df8b0cbf15bf58ca520a56e81df6e5a`.
- **Current CLI version:** `0.1.0`.
- **Release posture:** blocked; schema-1 release evidence cannot authorize a
  release.
- **Mutation posture:** explicit updater manager execution is enabled behind
  exact apply/confirmation/transaction gates; uninstall, cleanup, module
  install/activation, and third-party execution remain disabled.

This document is the public-safe starting point for future work. Read it before
changing code, then follow the topic-specific contracts linked below. Historical
runtime evidence, host-local paths, and operator-only details belong outside the
public repository.

The 2026-08-01 continuation added explicit TUI refresh/live update discovery,
long-list position/jump behavior, safer compact previews, consistent CLI
format/option parsing, and a bounded updater apply lane. Update execution is
still not implicit: live evidence, exact manager identity, explicit network-
write approval, initialized private state, short-lived confirmation, journal,
receipt, and fresh verification are mandatory. See the private project
implementation record for validation evidence.

## Why the last implementation changed direction

The original architecture kept the inventory collector in a separate
development binary while the installed core emitted an empty report. That kept
the boundary narrow but made the installed Mac product look unchanged. The
collector library is now a deliberate built-in, read-only foundation adapter.
The separate development binary and module lifecycle manifest remain useful for
fixtures and isolation work.

A second UX correction removed a redundant `uninstall options` navigation
section. Installed software is the user object; actions belong on each software
row. The TUI therefore has one canonical software list. Each row shows details
and only the uninstall posture that actually applies to that record.

Do not reintroduce an empty installed product or parallel action-specific lists
for the same software objects.

## Current user-visible product

### Launch and routing

- Bare `rz0` opens the full-screen TUI only when stdin and stdout are terminals
  and automation is not detected.
- `rz0 --tui` explicitly requires that interactive environment.
- `rz0 --no-tui`, `rz0 --json`, subcommands, pipes, redirects, and automation
  remain scriptable.
- The TUI restores raw mode, cursor visibility, and the alternate screen on
  normal exit and panic unwinding.

### TUI information architecture

The current navigation has five sections:

1. **overview** — live software count, available uninstall-review count, and
   basic interaction help;
2. **local store** — runtime.zero store, registry, and receipt state;
3. **installed software** — the canonical local software list with per-item
   options;
4. **modules** — installed and planned first-party module posture;
5. **safety gates** — mutation, trust, and execution boundaries.

Within **installed software**:

- every row offers local details;
- protected system software offers no uninstall command;
- Homebrew records offer a manager-owned uninstall review;
- local/user application bundles offer a quarantine-first uninstall review;
- unknown or unsupported ownership offers no uninstall command;
- Enter opens a preview; it never executes the command;
- selection follows long lists as the details panel scrolls.

The command rail includes `rz0 apps`, `rz0 uninstall plan <id>`, scan, doctor,
and automation-oriented dashboard commands. It previews commands but does not
run them.

### Scriptable commands

The principal current surfaces are:

```text
rz0
rz0 --tui
rz0 --no-tui
rz0 --json
rz0 doctor [--format json]
rz0 apps [--format json]
rz0 uninstall plan <installed-software-id> [--format json]
rz0 scan --dry-run [--include-raw-paths] [--format json]
rz0 updates --dry-run --fixture <updater-evidence.json> [--plan] [--queue] [--format json]
rz0 updates --dry-run --manager <id> --manager-output <path> --executable <path> [--plan] [--queue] [--format json]
rz0 updates --dry-run --probe --manager <id> --executable <path> --allow-network-read [--plan] [--queue] [--format json]
rz0 updates --apply --probe --manager <id> --executable <path> --allow-network-read --allow-network-write (--action <id> | --all) [--accept-no-rollback] [--challenge-issued-unix-seconds <unix-seconds>] [--confirm <phrase>]
rz0 modules [--format json]
rz0 modules --from <dir> [--format json]
rz0 modules validate <manifest.json> [--format json]
rz0 modules install --dry-run <package> [--format json]
rz0 store plan [--format json]
rz0 store status [--store-root <path>] [--format json]
rz0 store init --dry-run [--format json]
rz0 store init --yes [--format json]
```

`store init --yes` writes only runtime.zero-owned user-local scaffolding.
`updates --apply` is the first product system-mutation surface; it is limited to
exact live manager actions and requires the separate gates described below.

## Current Mac inventory behavior

The installed core calls the first-party inventory library with application
collection enabled and version probing disabled. Collection is shallow,
bounded, and read-only.

### Application bundles

Known roots are:

- `/System/Applications`;
- `/System/Applications/Utilities`;
- `/Applications`;
- `/Applications/Utilities`;
- the current user's direct `Applications` directory when its absolute home
  location is available.

The adapter:

- inspects direct entries only;
- accepts direct directories ending in `.app`;
- rejects symlinked roots and symlinked records;
- caps normalized application output at the shared 4,096-record ceiling;
- reads only a direct `Contents/Info.plist` up to 2 MiB for
  `CFBundleShortVersionString` or `CFBundleVersion`;
- omits publisher claims when no trusted publisher evidence exists;
- creates deterministic report-local IDs from normalized name/root evidence.

### Homebrew metadata

The adapter reads direct package directories under the `Cellar` and `Caskroom`
children of the standard Apple Silicon and Intel Homebrew prefixes. It does not
execute `brew`, use a shell, contact a network, resolve update availability, or
approve a manager action.

The reported version is the lexically last bounded direct version directory.
That is useful inventory evidence but is not proof of the active linked keg or
an update decision.

### Catalog and privacy

`rz0 apps` maps inventory records into a path-free
`installed_software_catalog`. It includes software ID, display name, version,
kind, scope, and uninstall posture. It omits install locations.

`rz0 scan --dry-run` emits the full shared `inventory_report`. Paths use
report-local redaction tokens by default. `--include-raw-paths` is an explicit
local-only override and raw output is not suitable for automatic export.
Application names and versions can still be sensitive and must be reviewed
before sharing.

`rz0 --json` deliberately uses a private dashboard model that omits software
names and reports `inventory_status: "private summary"`. Use `rz0 apps --format
json` when a local software catalog is explicitly required.

## Current uninstall behavior

`rz0 uninstall plan <id>` produces an `uninstall_review`, not an executable
action plan. It is dry-run-only and always reports:

- `writes_attempted: false`;
- confirmation and rollback requirements where applicable;
- `product_execution_authorized: false`.

Current mapping:

| Evidence | Scope | User-visible posture |
| --- | --- | --- |
| `/System/Applications/...` | system | protected; details only |
| `/Applications/...` | local | quarantine-first review |
| user Applications bundle | user | quarantine-first review |
| Homebrew Cellar/Caskroom | manager | manager uninstall review |
| unknown source/location | unknown | unsupported; details only |

Review IDs are validated before inventory is collected or echoed, preventing
control-sequence injection through the CLI error path.

### Why execution is still blocked

A review is not permission to remove software. Production Mac execution still
needs all of the following:

1. an exact reviewed manager executable identity-to-spawn primitive;
2. race-resistant, root-relative bundle identity and move/quarantine semantics;
3. exact source/finding/action-plan binding rather than a UI-only review;
4. confirmation challenge creation and durable single-use consumption;
5. transaction journal, receipt, registry-last publication, and rollback;
6. cancellation checks at every process and write boundary;
7. process-tree/resource/network/elevation policy;
8. interruption, tamper, conflict, and real power-loss evidence on disposable
   hosts.

Do not add `brew uninstall`, direct recursive deletion, Finder/AppleScript
trash calls, shell execution, or path-only bundle moves as a shortcut around
these gates.

## Foundation ownership map

Shared stability, security, privacy, and execution policy belong in foundation
crates. Domain modules may narrow these contracts but must not duplicate or
weaken them.

| Foundation package | Current responsibility | Important continuation boundary |
| --- | --- | --- |
| `validation-contract` | Canonical IDs, versions, hashes, references, and path grammar | Lexical validity never grants authority |
| `resource-contract` | Shared document, record, process, privacy, and evidence ceilings | Modules may narrow, never expand |
| `capability-contract` | Exact capability vocabulary and family partitions | Read/action grants cannot be mixed |
| `error-contract` | Stable machine codes and privacy/retry posture | Unknown errors fail closed; no automatic retry |
| `configuration-contract` | Immutable offline/default-deny schema-1 settings | Configuration cannot authorize execution |
| `privacy-contract` | Bounded report-local redaction and sensitive classes | No duplicate retained raw sensitive values |
| `diagnostics-contract` | Private config-bound doctor reports | No host/user/current-directory/environment-value disclosure |
| `inventory-contract` | Strict inventory shape, validation, bounds, and export privacy | Evidence is not an action or trust decision |
| `finding-contract` | Path-free typed findings and protected-data policy | Findings cannot authorize mutation |
| `support-contract` | Privacy-reviewed support input and summary output | Summaries cannot authorize release or execution |
| `action-plan` | Finding-bound update/uninstall/quarantine/restore plans | Plans remain dry-run artifacts; the core updater executor consumes one exact action |
| `confirmation-contract` | Plan-specific five-minute challenge and durable consumption | Confirmation evidence is necessary but not authority by itself |
| `cancellation-contract` | First-writer-wins cancellation and monotonic deadlines | A token is a signal, never spawn/kill/write authority |
| `secure-fs` | Opened-directory state I/O, locks, privacy, sync, publication | Windows mutation remains blocked pending runtime ACL proof |
| `artifact-identity` | Same-handle digest/identity and partial spawn leases | macOS exact handle-to-spawn remains unresolved |
| `process-host` | Shared bounded drain and Unix descriptor audit | Production containment and Windows handle audit remain incomplete |
| `module-trust` | Public-test-key signature and immutable staging contracts | No production key or trust root exists |
| `module-protocol` | Bounded no-execution protocol and blocked gate assessment | Only explicit test helpers execute |
| `module-lifecycle` | Eight digest-bound lifecycle transition plans | Schema 1 remains planning-only |
| `registry-contract` | Canonical installed-state records and bytes | Foundation owns ordering, paths, and digests |
| `transaction-contract` | Hash-chained journals, receipts, commit order, recovery | Real rollback execution and power-loss proof remain open |
| `performance-contract` | Frozen final-artifact budgets and evidence validation | Evidence cannot authorize release |
| `release-contract` | Exact target × seven-module × 12-stage ledger | Schema 1 is structurally blocked and non-authorizing |

## Module status

| Family | Source package | Current state | Missing before production support |
| --- | --- | --- | --- |
| Inventory/environment | `modules/inventory` | Library embedded for bounded reads; separate fixture/development binary | Broader managers/services/persistence and full platform runtime parity |
| Updater | `modules/updater` | Synthetic/captured-output parsers, live bounded probes, dry-run action-plan binding, serial queue, and explicit core manager apply lane | Platform-specific runtime proof, native rollback, Windows containment, and production acceptance matrix |
| Uninstall | `modules/uninstall` | Synthetic manager classifier; core has Mac review UX | Finding/action-plan integration and safe platform execution |
| Leftovers | `modules/leftovers` | Synthetic exact-runtime-owned classifier | Live ownership adapters and quarantine execution |
| Cache management | `modules/cache` | Synthetic ownership/exact-evidence classifier | Live adapters, risk budgets, quarantine/restore execution |
| Security/integrity | `modules/security-integrity` | Synthetic exact-digest report-only classifier | Trusted baselines, live reads, incident policy; no remediation authority |
| Report/export | `modules/report-export` | Strict stdin/stdout summary development binary | Lifecycle/core integration and final-artifact runtime proof |

All seven manifests remain planned source artifacts. The built-in inventory
library is not equivalent to lifecycle installation or module execution.

## Known product limitations and design debts

These are intentional continuation facts, not hidden production claims:

- The catalog is not literally every installed component. It does not yet cover
  Apple package receipts, MacPorts, Nix, language package managers, services,
  launch agents/daemons, browser extensions, drivers, or persistence entries.
- A Homebrew cask and its installed `.app` remain separate evidence records.
  The catalog now assigns deterministic identity groups and explicitly labels
  heuristic/disputed version relationships without merging provenance or
  authorizing an action.
- The TUI takes one inventory snapshot at launch and supports an explicit `r`
  refresh. Bounded `/` search, `f` filter cycling, and `s` sort cycling now
  operate on the cached snapshot without triggering a new scan. Long detail
  lists expose the current item position while scrolling.
- Per-item TUI options currently mean details plus uninstall posture and update
  availability. The scriptable updater apply lane is live; TUI mutation controls
  and repair/integrity/export/cleanup execution are not yet wired.
- Application publisher identity is unknown unless a future trusted adapter
  provides it.
- Bundle IDs are deterministic evidence IDs, not permanent global product IDs;
  moving an app between roots can change its ID.
- Homebrew versions come from directory metadata and may not identify the active
  linked version.
- The performance contract directly benchmarks scan, not the separate `apps`
  command. Scan exercises the same collector; add explicit catalog/TUI startup
  operations in a future schema revision rather than silently changing the
  exact schema-1 operation set.
- The plain text dashboard can expose local software names; machine-readable
  dashboard JSON intentionally does not.
- Unsigned macOS artifacts remain subject to Gatekeeper warnings and carry no
  notarization claim.

## Validation snapshot at pause

The final pause snapshot passed:

- `cargo fmt --check`;
- 299 default workspace tests;
- 310 all-feature workspace tests;
- strict native Clippy over all targets/features;
- Windows MSVC x86, x86-64, and ARM64 target Clippy/checks;
- Linux GNU x86-64 and ARM64 target Clippy/checks;
- Intel macOS target Clippy/checks;
- pinned Windows-7-baseline x86/x86-64 build-std checks;
- RustSec scan with 1,173 advisories loaded and no known vulnerability in 150
  lock entries;
- cargo-deny policy, with only the documented Ratatui `hashbrown` duplicate and
  unmatched `BSD-3-Clause` allowance warnings;
- Markdown links, formatting, diff, and secret-pattern scans;
- deterministic universal2 ZIP construction;
- ARM64 and Rosetta x86-64 catalog execution with matching record totals;
- four-case PTY smoke through both universal slices;
- final-artifact performance evidence within the broad schema-1 budget;
- unsigned DMG verification.

Workspace resolution at pause: 31 local packages, 119 external packages.

### Final artifact identifiers

These artifacts were locally built from the verified product implementation
commit and were not
published:

- deterministic universal2 ZIP SHA-256:
  `7744b5a6e408785e523b168897221e48b896f22c01a6707622696855accffc8b`;
- universal binary SHA-256:
  `696138870ac2430d2017405470e0b463afb6e2ffd44f7461972c703119cd139a`;
- unsigned DMG SHA-256:
  `d4c29149411ffb4fd7b25f2356b80dbe730155a8e7da0fb03c51b4ad585105fa`.

Artifact evidence is descriptive and non-authorizing. Local temporary artifact
locations are not durable release storage.

## Dependency-ordered continuation plan

Resume in this order unless new evidence changes a dependency:

### 1. Reproduce the pause snapshot

1. Read `AGENTS.md` and this document.
2. Confirm a clean `main`, fetch, and fast-forward pull.
3. Verify the expected source commit or review every intervening commit.
4. Run format, focused tests, full workspace tests, strict Clippy, and live
   `rz0 apps`/dry-run scan.
5. Reinstall the local development binary only after checks pass.
6. Do not treat old `/tmp` artifact evidence as durable release input.

### 2. Stabilize software identity and catalog UX

1. Define foundation-owned multi-source software identity/provenance without
   merging records merely by display name.
2. Add synthetic reconciliation fixtures for app/cask duplicates, disagreement,
   renamed apps, multiple versions, missing plist data, and moved roots.
3. Complete the software identity/provenance reconciliation fixtures while
   preserving bounded memory, no background daemon, explicit refresh semantics,
   and private JSON behavior.
4. Add explicit performance operations only through a schema-compatible or new
   schema contract.

### 3. Complete production process and filesystem prerequisites

1. Implement or formally reject an exact macOS executable identity-to-spawn
   primitive.
2. Complete production process-tree containment, descriptor/handle policy,
   resource enforcement, cancellation, and teardown.
3. Design race-resistant root-relative bundle quarantine/restore with exact
   identity, no recursive delete, and durable rollback evidence.
4. Prove interruption, conflict, tamper, symlink, hardlink, cross-filesystem,
   permission, and power-loss behavior on disposable filesystems.

### 4. Bind reviews to the foundation action pipeline

1. Convert exact catalog evidence into shared finding reports.
2. Bind each review to a finding ID, sealed report digest, action plan, and
   write set.
3. Require exact confirmation, durable single-use consumption, transaction
   journal, receipt, registry-last commit, and cancellation.
4. Execute only manager-native or quarantine-first methods whose platform gates
   have real runtime evidence.
5. Keep protected/unknown software blocked.

### 5. Expand live domain behavior equally

Implement updater, uninstall, leftovers, cache, integrity, and report/export
behind the shared foundation. Do not create module-local validation, process,
transaction, privacy, retry, cancellation, or trust systems.

### 6. Complete equal platform and artifact matrices

Use final-artifact-only disposable hosts for Windows, Server, Linux, older/Intel
macOS, terminals, shells, filesystems, accessibility, interruption, and power-
loss evidence. Build runners may contain source/toolchains; compatibility hosts
must not.

### 7. Review external release actions separately

Public workflows, GitHub releases, package submissions, deployment, signing,
paid enrollment, production credentials, recurring automation, and non-
disposable-host mutation remain explicit approval events.

## First-session command checklist

Use the repository's pinned/current toolchain arrangement rather than assuming
Cargo is globally available:

```bash
git status --short --branch
git fetch --prune origin
git pull --ff-only origin main
cargo fmt --check
cargo test --workspace --locked
cargo test --workspace --locked --all-features
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo run -- apps --format json
cargo run -- scan --dry-run --format json
cargo run -- doctor --format json
git diff --check
```

Then run only the cross-target, audit, packaging, PTY, performance, and DMG lanes
that are relevant to the resumed change. A release claim still requires all
frozen acceptance cells, not only this smoke list.

## Canonical documentation map

Read in this order for a broad restart:

1. this status/resumption guide;
2. [`roadmap.md`](roadmap.md);
3. [`production-readiness.md`](production-readiness.md);
4. [`architecture.md`](architecture.md);
5. [`module-system.md`](module-system.md);
6. [`inventory-schema.md`](inventory-schema.md);
7. [`tui.md`](tui.md);
8. [`action-planning.md`](action-planning.md);
9. [`module-trust-and-execution.md`](module-trust-and-execution.md);
10. [`release-acceptance.md`](release-acceptance.md).

Topic contracts:

- safety limits and validation: [`resource-contract.md`](resource-contract.md),
  [`validation-contract.md`](validation-contract.md),
  [`error-contract.md`](error-contract.md), and
  [`capability-contract.md`](capability-contract.md);
- privacy/configuration/diagnostics:
  [`privacy-contract.md`](privacy-contract.md),
  [`configuration-contract.md`](configuration-contract.md), and
  [`diagnostics-contract.md`](diagnostics-contract.md);
- findings/actions/confirmation:
  [`finding-contract.md`](finding-contract.md),
  [`action-planning.md`](action-planning.md), and
  [`confirmation-contract.md`](confirmation-contract.md);
- state/recovery/filesystem:
  [`transaction-journal.md`](transaction-journal.md),
  [`installed-registry-contract.md`](installed-registry-contract.md), and
  [`secure-filesystem.md`](secure-filesystem.md);
- process/trust/lifecycle:
  [`artifact-identity.md`](artifact-identity.md),
  [`process-host-foundation.md`](process-host-foundation.md),
  [`module-process-protocol.md`](module-process-protocol.md),
  [`signature-verification.md`](signature-verification.md), and
  [`module-lifecycle-contract.md`](module-lifecycle-contract.md);
- support/performance/release:
  [`support-report-contract.md`](support-report-contract.md),
  [`performance-contract.md`](performance-contract.md),
  [`release-packaging.md`](release-packaging.md), and
  [`support-policy.md`](support-policy.md).

## Invariants to preserve

- Report first, dry-run first, quarantine first, explicit confirmation first.
- One canonical software list with per-object options; do not duplicate the same
  objects into action-specific navigation sections.
- No secret, credential, session, private key, host identity, or private path in
  public evidence.
- No direct recursive deletion of applications or unknown data.
- Manager-native actions before filesystem cleanup.
- No macOS process execution until exact identity-to-spawn is reviewed.
- No weaker path-based Windows mutation fallback.
- No automatic retry in schema 1.
- Evidence, plans, leases, receipts, approvals, diagnostics, and ledgers never
  authorize mutation or release by themselves.
- Every platform and all seven module families remain release-critical.
