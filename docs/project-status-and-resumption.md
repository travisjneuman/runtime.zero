# Project Status and Resumption Guide

## Snapshot identity

- **Reviewed:** 2026-08-21.
- **Product status:** active pre-alpha development; not production-ready and not
  a supported release.
- **Canonical branch:** `main`.
- **Reviewed source baseline:**
  `c6118ada328da0bc6e410e09a979752a06b96dde` (`feat: expose effective configuration review`).
- **Current behavior implementation:**
  `c6118ad` on `main`, including the redesigned TUI, Rust toolchain contract,
  AIUP updater-provider adapter, bounded cache/leftovers evidence review,
  fixture and bounded exact-file integrity evidence, receipt-bound local
  recovery completion, explicit provider ownership in Toolchain rows, the
  pre-start Windows Job Object/handle-list process host, shared version probes,
  independent portable-package verification, the read-only local test-key-bound
  module package trust review, and optional complete-file-set package
 enumeration with bounded undeclared-file rejection.
  Bounded provenance consistency checks now reject malformed or publisher-drifted
  package metadata without treating provenance as trust authority. The leftovers
  surface now also has an explicit one-file module-store plan and a separate
  confirmation-bound quarantine invocation. The cache surface now has the same
  exact one-file plan/apply boundary for runtime.zero-owned cache artifacts,
  including a separate physical cache-root binding on macOS. The new exact
  `restore` command derives a fresh restore plan from one validated quarantine
  record and reuses the receipt-bound executor for an unoccupied original
  cache/module path; it does not provide recursive cleanup, uninstall,
  deletion, elevation, or module authority. The read-only `recovery --dry-run`
  command inventories bounded quarantine records and reports restore
  eligibility without exposing absolute host paths or adding a second mutation
  authority. It now also inspects bounded immutable transaction journal heads
  without acquiring writer locks, reports stable per-transaction recovery
  decisions and operator guidance, and surfaces checked/invalid/action-required
  journal counts plus read-only review-warning counts in the TUI Diagnostics
  workspace. It never publishes, completes, restores, rolls back, or adds a
  second mutation route. Cache review now emits a separate
  `cache_safety_policy` summary plus path-free per-root modification-time,
  age-threshold, scan-completeness, and conservative lock-marker evidence; the
  TUI renders the observation and age-threshold state without creating a second
  action route. The read-only `modules status` surface now composes registry and
  receipt, manifest, and declared package-file evidence, reports valid records
  as `installed_inactive`, reports missing or invalid evidence as `degraded`, redacts unsafe registry
  identifiers, and never claims activation or invocation authority.
  The TUI Diagnostics workspace now renders the same inactive/degraded counts
  and explicit lifecycle-unavailable state, while the JSON dashboard exposes
  those additive non-authorizing fields from the same status report.
  The follow-on TUI presentation slice keeps primary diagnostics and monitor
  rows concise and moves dense evidence into the selected explanation pane;
  the text and Ratatui renderers share the calmer `LOCAL SNAPSHOT` header copy.
  Module status now also checks the installed manifest and declared package
  files, so a valid registry-plus-receipt with missing or tampered module bytes
  is `degraded` rather than a false `installed_inactive` result.
  The bounded developer-only staging trial now verifies a locally selected
  read-only first-party package through held artifact identities and a detached
  public test-key envelope, requires an initialized private store and exact
  confirmation, stages verified bytes with transaction/commit/stage receipts,
  and leaves the installed registry unchanged. Its source path is redacted in
  reports; it never activates, invokes, executes, fetches, replaces, or grants
  production trust to module bytes.
  The explicit developer-promotion variant now publishes one
  test-key-only `installed_inactive` registry record and separate install
  receipt through the same commit coordinator for local lifecycle testing; it
  refuses replacement and still cannot activate code or grant production
  invocation authority. The separate developer-only invocation lane now accepts
  only a promoted first-party.inventory package with complete immutable file
  evidence, binds its Rust executable through the shared process host, and
  validates only a path-redacted read-only response; it does not activate state,
  write a lifecycle receipt, or run third-party code.
  The read-only `config` command exposes the same immutable built-in schema-one
  privacy, execution, mutation, and lifecycle policy that `doctor` binds by
  digest; it never loads user configuration or authorizes execution.
  The TUI Diagnostics selected-evidence pane now shows that same policy digest
  and a compact disabled-policy summary without adding a new action rail.
  The same read-only status report now reviews staged receipts and destination
  bytes as a separate `staged_modules` collection, and the redesigned TUI
  Diagnostics workspace exposes the staged count without treating it as an
  installed or active module. Each valid staged receipt is also bound to its
  immutable committed transaction-journal head and commit receipt; missing or
  tampered transaction evidence is degraded review data.
  The TUI Diagnostics workspace separately counts staged entries requiring
  review, so invalid staged evidence is not visually mixed with valid staging.
  AIUP-managed toolchain records remain explicitly `observed-only` in both the
  Rust report and TUI rows; provider availability review remains a separate
  plan/confirmation path. The Rust AIUP dry-run adapter now rejects output with
  no recognized tool or detected-version catalog section, so arbitrary success
  text and malformed tool labels cannot silently become an empty successful
  provider review.
- **CLI version:** `0.1.0`.
- **Release posture:** blocked; schema-1 release evidence cannot authorize a
  release.
- **Current writes:** explicit user-local Unix/Windows-guarded store scaffolding
  and a working macOS/Linux/Windows-pre-alpha manager-update executor exist,
  with Windows runtime evidence still absent. Uninstall, recursive cleanup,
  broad quarantine/restore, production module lifecycle execution, and
  third-party execution remain unavailable; the narrow exact-file leftovers and
  runtime-cache quarantine lanes, exact-record restore, and developer-only
  signed module staging are the only module-adjacent write paths.

The exact cache quarantine slice is pushed at `77d389a`, the leftovers slice at
`87aef29`, the
updater/receipt slices at `d5e5153`, `ee1a1eb`, and `ad999c3`, the compact TUI
posture update at `8132b4e`, the exact-file integrity slice at `47c6f9f`, and
the Windows process-host/probe slice at `39adb92`, the module trust review
slice at `a6664d6`, the complete-file-set slice at `3012176`, provenance
validation at `63f7d8d`, the exact leftovers plan/apply lane at `22c3619`, and
the cache-root/restore slice at `0ce2cc6`, the bounded recovery inventory at
`366153f`, and the TUI recovery evidence slice at `08635e6`.
The status refreshes are `ea4593a`, `da5c5a0`, and `7602a3f`; package
provenance was finally bound at `7d0ed91`; the bounded cache policy slice is
`6d98021`.
The bounded transaction journal inspection and TUI warning-count slice is
`74257e3`.
The path-redacted module lifecycle status slice is `baa7e61`.
The TUI lifecycle-status parity slice is `9b7e01d`.
The attention-first TUI evidence presentation slice is `1f94241`.
The installed-module-byte status hardening slice is `17dd2e2`.
The developer-only signed module staging slice is `1e782c8`.
The staged-receipt status and TUI count slice is `26c1b44`.
The staged-status evidence refresh is `fb95b45`.
The staged transaction-evidence cross-check slice is `f9e28cb`.
The staged-review TUI warning slice is `b818e54`.
The AIUP ownership-posture parity slice is `7a120fd`.
The AIUP malformed-catalog fail-closed slice is `c036f56`.
The developer-only installed-inactive promotion slice is `0fe633f`.
The developer-only first-party inventory process invocation slice is
`8f9f3c7`.
The effective configuration review slice is `c6118ad`.
Local
`main` and
`origin/main` matched after publication. The source validation baseline passes
`cargo fmt --all -- --check`, `cargo test --workspace --locked`, the full
`cargo test --workspace --locked --all-features` suite, strict all-features
workspace Clippy, Windows MSVC and Linux GNU cross-target `cargo check`, and
`git diff --check`. The current local aarch64 Apple Silicon package from
`c6118ada328da0bc6e410e09a979752a06b96dde` has binary SHA-256
`ceae92e5a9d0a1ccf44badec8350291106fc0c838f90f02d3fd90bd34244be6c`, ZIP
SHA-256
`5382ea225f9546b2e4cade3b4d823948c0e94e6d93dd8138652c00a3ba37c003`, and
1,891,076 bytes across 8 verified members. Embedded SBOM, third-party notices,
and artifact-manifest SHA-256 values are
`0424864a0fd6cf613f457ac3c462aa20f23080dbc010cfd19a03d01e2fdadb21`,
`ba8e6534e7fea9d48cf575f03cdccf13125b850354ffc7ba6005af8dd6f81b7a`, and
`9171307c5431c5800852e1b68c6e3646cf329460451a0543e1395b67e77e6bf7`; their
embedded sizes are 160,426, 290,906, and 977 bytes respectively. Independent
verification, four PTY terminal smoke cases, and ten-sample final-artifact
performance evidence passed against this source head. The artifact remains
unsigned and unnotarized until an owner-led signing/notarization lane exists.
It is a local Apple Silicon artifact, not a public release or cross-platform
runtime claim. The developer invocation process path is separately bounded and
does not change this release posture.
The terminal evidence IDs are `terminal:aarch64-apple-darwin-arm64-ceae92e5a9d0`
and `perf:aarch64-apple-darwin-arm64-ceae92e5a9d0`. The packaged `config
--format json` review also passed with configuration digest
`b4d57157ae30be77f81a293bd49ddc2f939168377b20b9d9bb16a4ea1e40258f`.

The current release decision remains blocked. Fresh `doctor --format json`
reports 6 passing and 4 blocked policy checks; `scan --dry-run` is read-only
with no writes; and the bounded provider review reports 20 sources, 13 source
successes, 59 serial queue items, and one AIUP-managed candidate without
execution authorization. The same local `recovery --dry-run --format json`
review inspected 11 valid transaction journals, found 0 invalid journals, and
classified all 11 as action-required under the conservative assessment; one
bounded warning reported persistent writer-lock markers without claiming active
ownership. Linux release linking was re-attempted with the
installed Rust LLD and still fails because this macOS host lacks the target
Linux C-runtime libraries. Windows `link.exe` is likewise unavailable. These
checks do not substitute for target-native runtime, signing, accessibility,
recovery, or owner-acceptance evidence.

For the product end state, module contract, enable/disable semantics, delivery
waves, and next-shift checklist, see
[`engineering-handoff.md`](engineering-handoff.md). For document precedence and
the full topic map, see [`documentation-index.md`](documentation-index.md). The
2026-07-30 pause handoff and earlier plans remain historical evidence only.

## Executive assessment

`runtime.zero` now has a broad provider-driven product surface and a working
pre-alpha updater executor. It remains far from a defensible 1.0 release because
full platform source parity, OS capability isolation, rollback, manager-specific
recovery beyond local journal finalization, module trust/lifecycle, uninstall/cleanup execution, accessibility,
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
- exact local module manifest/package-file hashing plus detached public
  test-key verification, with identity/version/digest binding and no execution
  authority, plus optional bounded complete-file-set enumeration;
- deterministic local packaging/SBOM/notice generation, shell completions, a
  manual page, and operator guides.

The largest immediate risks are:

- macOS uses a last-moment direct-path identity/digest binding because Darwin
  exposes no public fexecve-style primitive; this is weaker than Linux's held
  descriptor launch and remains pre-alpha;
- Windows updater execution now uses the production Rust process host with
  pre-start Job Object assignment and an explicit inherited-handle list; real
  Windows runtime, reparse/ACL, cancellation, and capability-isolation proof
  remain incomplete;
- Unix process groups are containment aids, not syscall/filesystem/network/
  privilege sandboxes, and a hostile child may attempt session escape;
- cancellation is integrated into confirmed execution but not every discovery,
  verification, and write boundary;
- a valid external-effect receipt can identify an interrupted final journal
  commit, and a fresh receipt-bound recovery-completion command now exists;
- native rollback and disposable-host power-loss/
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
rz0 config [--format text|json]
rz0 apps [--format text|json]
rz0 cache --dry-run [--format text|json] [--fixture <cache-input.json>]
rz0 cache --dry-run --plan --path <absolute-cache-file> [--format text|json]
rz0 cache --apply --path <absolute-cache-file> [--challenge-issued-unix-seconds <seconds>] [--confirm <phrase>] [--format text|json]
rz0 leftovers --dry-run [--format text|json] [--fixture <leftover-input.json>]
rz0 leftovers --dry-run --plan --path <absolute-module-file> [--format text|json]
rz0 leftovers --apply --path <absolute-module-file> [--challenge-issued-unix-seconds <seconds>] [--confirm <phrase>] [--format text|json]
rz0 recovery --dry-run [--format text|json]
rz0 restore --dry-run --plan-id <exact-quarantine-plan-id> [--format text|json]
rz0 restore --apply --plan-id <exact-quarantine-plan-id> [--challenge-issued-unix-seconds <seconds>] [--confirm <phrase>] [--format text|json]
rz0 integrity --dry-run --fixture <integrity-input.json> [--format text|json]
rz0 integrity --dry-run --path <absolute-file> --sha256 <digest> [--format text|json]
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
rz0 updates --recovery-complete --transaction <id> [--challenge-issued-unix-seconds <seconds>] [--confirm <phrase>] [--format text|json]
rz0 updates --apply --probe --manager <id> --executable <path> --allow-network-read --allow-network-write (--action <id> | --all) [--accept-no-rollback] [--challenge-issued-unix-seconds <seconds>] [--confirm <phrase>] [--format text|json]
rz0 updates --apply --all-providers --allow-network-read --allow-network-write [--accept-no-rollback] [--format text]
rz0 updates --apply --all-providers --allow-network-read --allow-network-write --action <id> [--accept-no-rollback] [--challenge-issued-unix-seconds <seconds>] [--confirm <phrase>] [--format text|json]
rz0 modules [--from <directory>] [--format text|json]
rz0 modules status [--store-root <path>] [--format text|json]
rz0 modules validate <manifest.json> [--format text|json]
rz0 modules install --dry-run <package> [--format text|json]
rz0 modules install --developer-trial --dry-run <package> --signature <envelope.json> --trusted-test-key <key.json> --store-root <path> [--format text|json]
rz0 modules install --developer-trial --apply <package> --signature <envelope.json> --trusted-test-key <key.json> --store-root <path> --challenge-issued-unix-seconds <seconds> --confirm <exact-phrase> [--format text|json]
rz0 modules invoke --developer-trial --dry-run --module-id first-party.inventory --store-root <path> [--format text|json]
rz0 modules invoke --developer-trial --apply --module-id first-party.inventory --store-root <path> --challenge-issued-unix-seconds <seconds> --confirm <exact-phrase> [--format text|json]
rz0 modules trust verify --manifest <manifest.json> --signature <envelope.json> --trusted-test-key <key.json> [--format text|json]
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
| `config` | No | No | Implemented immutable built-in effective-policy review; never authorizes execution |
| `apps` | No | No | Implemented path-free catalog; names/IDs remain sensitive |
| `cache --dry-run` | No | No | Bounded known-root ownership review; no cleanup authority |
| `cache --dry-run --plan --path FILE` | No | No | One exact runtime-cache-file plan with digest/size binding; no move |
| `cache --apply --path FILE` without `--confirm` | No | No | Prints a short-lived exact challenge; no write |
| `cache --apply --path FILE --confirm PHRASE` | No | Yes, one exact quarantine transaction | Confirmation-bound foundation move only; no recursion, deletion, elevation, or network |
| `leftovers --dry-run` | No | No | Bounded runtime.zero-owned module/log and unreferenced-receipt review; no quarantine authority |
| `leftovers --dry-run --plan --path FILE` | No | No | One exact module-store file plan with digest/size binding; no move |
| `leftovers --apply --path FILE` without `--confirm` | No | No | Prints a short-lived exact challenge; no write |
| `leftovers --apply --path FILE --confirm PHRASE` | No | Yes, one exact quarantine transaction | Confirmation-bound foundation move only; no recursion, deletion, elevation, or network |
| `recovery --dry-run` | No | No | Bounded quarantine-record plus immutable transaction-journal inventory with logical identity, conservative decisions, operator guidance, and no absolute host paths |
| `restore --dry-run --plan-id ID` | No | No | Reads one validated quarantine record and builds a fresh exact restore plan |
| `restore --apply --plan-id ID` without `--confirm` | No | No | Prints a short-lived exact restore challenge; no write |
| `restore --apply --plan-id ID --confirm PHRASE` | No | Yes, one exact restore transaction | Restores only the validated payload to its original unoccupied cache/module path; no overwrite, recursion, deletion, elevation, or network |
| `integrity --dry-run` | No | No | Fixture or bounded exact-file digest review; no trusted baseline or remediation |
| `scan --dry-run` | No | No | Implemented; paths redacted by default |
| `monitor` | No | No | Implemented one-shot native snapshot; depth varies |
| `report` | No | No | Implemented privacy-reviewed summary; external sharing never auto-authorized |
| TUI startup/`r` | No | No | Implemented inventory/monitor refresh |
| TUI `u` / updater `--probe` | Manager may read remote metadata after acknowledgement | No product write | Bounded provider review/probe |
| TUI `U` selected update | Provider metadata plus manager network write where required | Manager plus private journal/receipt writes | Direct shared macOS/Linux/Windows process-host flow with exact TUI confirmation; Windows remains pre-alpha |
| `updates --all-providers` | Providers may read remote metadata after acknowledgement | No product write | Provider-driven bounded review across installed managers, language environments, self-updaters, and declared app metadata; missing, observed-only, and unsupported sources remain warnings |
| updater fixture/captured output | No | No | Implemented review/planning |
| `updates --recovery-status` | No | No | Implemented deterministic evidence assessment only |
| `updates --recovery-complete` | Runtime.zero private journal only | No manager rerun | Implemented fresh receipt-bound local finalization; no rollback or automatic mutation |
| `updates --apply` | Explicit read/write acknowledgement; not OS-isolated | Manager plus private journal/receipt writes | Working macOS/Linux/Windows pre-alpha lane with receipts; Windows runtime/ACL/reparse proof remains open |
| uninstall plan | No | No | Shared finding and optional sealed action plan; no execution |
| module validation/install planning | No | No | Implemented planning only; optional complete-file-set review rejects undeclared files |
| `modules install --developer-trial` | Local package read plus explicit runtime.zero-owned write | Staged bytes and transaction/stage receipts; optional test-key-only installed-inactive registry/receipt with `--developer-promote` | Implemented developer-only staging/promotion; no activation, production trust, or public distribution |
| `modules invoke --developer-trial` | Local promoted package read plus bounded Rust child process | No registry/lifecycle write; path-redacted inventory response and executable binding evidence | Implemented only for promoted first-party.inventory; no activation, lifecycle receipt, sandbox, third-party execution, or production authority |
| `modules trust verify` | No | No | Test-key-only exact package review; no production trust root or lifecycle authority |
| `modules status` | No | No | Path-redacted registry/receipt plus developer-staging view; reports staged, installed-inactive, or degraded, never active |
| store plan/status | No | No | Implemented read-only inspection |
| `store init --yes` | No | Runtime.zero-owned user-local scaffold | Unix and guarded Windows owner/DACL path; runtime acceptance remains open |
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
TUI action set because recovery requires a separate receipt-bound challenge and
is intentionally CLI-only.

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
before spawn. Windows uses the pre-start Job Object/explicit handle-list host
and remains open only for runtime, ACL/reparse, and broader capability proof.
Elevated Unix manager actions use non-interactive `/usr/bin/sudo`;
no password or interactive helper is collected. Known self-updaters may replace
their launcher and are verified through the declared transition plus fresh
provider evidence.

### Recovery assessment

`updates --recovery-status` validates the exact journal and receipt from the
private store and selects one conservative action: abort without writes, verify
an uncertain external effect, require explicitly approved final journal
completion, take no action for consistent committed evidence, or refuse
inconsistent evidence. `updates --recovery-complete` handles only the verified
receipt case with a fresh short-lived challenge, durable approval, and one
append-only local commit event. Neither path repairs, retries, rolls back, or
reruns a manager.

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

No uninstall process, recursive cleanup, deletion, elevation, dependent-package
review, or uninstall verification/rollback/recovery occurs. The exact
leftovers lane is a separate single-file quarantine path: it requires a
freshly recomputed digest-bound plan and short-lived confirmation, and it uses
the foundation transaction/receipt executor. A `planned` action still has
`execution_authorized: false`; no uninstall executor exists.

## Module catalog and lifecycle

All seven first-party manifests remain `planned`:

| Family | Current implementation | Major missing work |
| --- | --- | --- |
| Inventory/environment | Embedded read-only collector plus development binary | Full source/platform parity and signed lifecycle |
| Updater | Provider-driven plans plus working macOS/Linux core executor | Windows isolation, rollback/recovery, manager/runtime matrix, release proof |
| Uninstall | Shared synthetic/live findings and dry-run manager plans | Every execution/elevation/quarantine/rollback path |
| Leftovers | Synthetic exact-runtime-owned classifier plus one exact module-file plan/apply lane | Post-uninstall ownership discovery, broad cleanup, platform parity, retention, and full quarantine/restore |
| Cache | Synthetic ownership-aware classifier plus bounded live review, explicit age/size/lock-marker policy evidence, and one exact-file quarantine/restore path | Platform-native active-use/ownership proof, retention/conflict policy, multi-file lifecycle, platform parity, and full acceptance |
| Security/integrity | Fixture and bounded exact-file digest classifier | Trusted baselines, incident review, and remediation policy |
| Report/export | Strict module binary plus integrated foundation report | Signed lifecycle and final-artifact platform proof |

The core can validate manifests/hashes, plan installation, and run the one
explicit developer-trial first-party.inventory invocation boundary. It cannot
install, activate, repair, migrate, upgrade, deactivate, or uninstall modules,
and the developer invocation does not provide production trust, sandboxing,
third-party execution, or lifecycle authority. Test-key signatures, schemas,
fixtures, process tests, and lifecycle plans remain non-production evidence.

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
| `process-host` | Bounded direct transport, Unix groups, and Windows pre-start Job Object/handle list | OS sandbox, runtime proof, and broader capability policy |
| `secure-fs` | Opened-directory state I/O | Windows ACL creation/runtime and FS matrix |
| `artifact-identity` | Same-handle identity plus Linux lease and macOS path-revalidation binding | Windows production binding and cross-platform runtime proof |
| `module-trust` | Test-key signature/staging contracts, local review adapter, complete-file-set package review, and bounded provenance consistency | Production trust roots/freshness/transparency/revocation |
| `module-protocol` | Unauthorized preview/test child | Production module host |
| `module-lifecycle` | Eight planning transitions | No lifecycle execution |
| `registry-contract` | Canonical installed state | No module install publication |
| `transaction-contract` | Journal, external receipts, coordinator, recovery | Exact domain rollback/platform proof |
| `performance-contract` | Nine read-only command budgets | TUI timing and target-native evidence |
| `release-contract` | Target × module × stage ledger | RC freeze/evidence population |

## Validation baseline

Current source validation for `c6118ada328da0bc6e410e09a979752a06b96dde` and
the packaged artifact on `aarch64-apple-darwin`:

- `cargo fmt --all -- --check` passed;
- `cargo test --workspace --locked` and the full
  `cargo test --workspace --locked --all-features` suite passed, including the
  module-status fixture/TUI parity, missing-module-byte, and concise-selected-evidence cases plus module-trust, complete-file-set,
  provenance-consistency, process-host, transaction, and TUI cases;
- strict locked all-target all-features Clippy passed with `-D warnings`;
- Windows MSVC cross-target `cargo check --workspace --target
  x86_64-pc-windows-msvc --locked` passed;
- Linux GNU cross-target `cargo check --workspace --target
  x86_64-unknown-linux-gnu --locked` passed;
- the Ratatui buffer matrix passed for all five workspaces at 58x16, 80x24,
  118x30, and 160x50 in plain and color modes;
- final-artifact `doctor`, `scan --dry-run`, `cache --dry-run`, and
  `recovery --dry-run` JSON reviews passed privacy checks; recovery exposed
  bounded journal decisions and review warnings without raw paths, while cache
  exposed the policy, age, scan, and active-use uncertainty fields;
- `git diff --check` passed;
- the trust fixture review returned a valid package/signature result while
  retaining `test_key_only: true`, `execution_authorized: false`, and
  `writes_attempted: false`;
- the Apple Silicon ZIP was independently verified from this exact package
  source head; four PTY cases and ten-sample final-artifact performance evidence
  passed, and its hashes/runtime evidence are recorded in the private
  release-artifact note. The package verifier reported 8 members and pass.

Historical earlier validation also recorded completion-source parity, Bash/Zsh
syntax, PowerShell parsing, `mandoc`, Markdown-link checks, module-manifest
checks, secret/private-path scans, and diff hygiene. Fish and ShellCheck were
unavailable in that lane, so Fish had static coverage rather than a native
parser run.

`cargo-audit` and `cargo-deny` were unavailable and were not auto-installed.
Locked metadata still resolved 150 packages (31 workspace and 119 external), and
native target-filtered release metadata covered 119 reachable packages (96
external). Older continuation records also cover `cargo check --workspace`,
doctor, scan, and development-Mac updater smoke evidence. No uninstall,
Cloudflare/site mutation, release publication, or production release action
was run in those lanes.

## Known limitations

### Product and UX

- No stable 1.0 CLI/JSON/API compatibility guarantee exists.
- TUI update actions now enter the shared direct confirmation/execution flow;
  uninstall and cleanup actions remain reviews rather than execution flows.
- No uninstall, cleanup, broad restore, integrity remediation, or module
  lifecycle execution exists; exact-record restore is limited to the narrow
  receipt-bound cache/module lane described above, and recovery inventory is
  read-only.
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
   capabilities/network/elevation, full cancellation, manager-specific recovery,
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
