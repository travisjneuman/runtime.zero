# Project Status and Resumption Guide

## Snapshot identity

- **Reviewed:** 2026-08-22.
- **Product status:** active pre-alpha development; not production-ready and not
  a supported release.
- **Canonical branch:** `main`.
- **Reviewed source baseline:**
  `eb7178a724063130b02d886879293009fc81dd47` (`Align TUI documentation with final frontend`).
- **Current behavior implementation:**
  `eb7178a724063130b02d886879293009fc81dd47` on `main`, including the Rust-first Dossier Queue TUI, typed
  presentation contract, Rust toolchain contract,
  provider updater adapters, bounded cache/leftovers evidence review,
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
  deletion, elevation, or module authority. Built-in Linux inventory also reads
  bounded Flatpak `active/metadata` records and preserves exact app
  ID/architecture/branch identity without granting action authority. The
  updater also has strict Zypper XML package-row, Flatpak JSON/ref/commit, and
  Snap five-column `refresh --list` parsers under the forced `C` locale; they
  retain only bounded human-readable version evidence and bind update
  candidates to their exact provider identity. The
  read-only `recovery --dry-run`
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
  the text and Ratatui renderers share the calmer `local snapshot` header copy
  and the Home workspace keeps its first frame to the next useful review step.
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
  Provider availability review remains a separate plan/confirmation path, and
  malformed provider output cannot silently become an empty successful review.
  The foundation now also exposes a narrow exact-manager
  uninstall apply lane: it derives a fresh manager-owned action plan, binds an
  allowlisted executable by digest and size, requires destructive confirmation
  plus explicit no-rollback acknowledgement, records the external effect in
  the shared transaction/receipt path, and requires fresh installed-software
  evidence after the manager returns. It does not provide dependent-package
  review, recursive cleanup, user-bundle handling, rollback, or production
  lifecycle authority.
  The shared update/uninstall challenge view now exposes the operation, manager,
  target, bounded command arguments, risk, elevation/network posture, sealed
  executable digest/size, capabilities, plan identity, expiry, rollback posture,
  and exact phrase before confirmation. Paths remain intentionally undisclosed;
  the executor still revalidates every bound input.
  Installed registry records now persist the foundation-owned
  `lifecycle_state: "installed_inactive"`; schema 1 rejects any other explicit
  state, and status consumes the persisted value without exposing activation.
  Newly generated install receipts carry the same state plus explicit false
  activation/invocation authority flags; an explicit active receipt is invalid
  evidence.
  The Toolchain CLI and TUI now consume the same bounded named-executable
  inventory as Scan, label executable-only records `observed-only`, and keep
  wrapper-like PATH names out of the toolchain surface. The renderer reserves
  the selected-context pane before truncating dense primary rows, so expanded
  evidence cannot push the key explanation panel off-screen. Home now derives
  its toolchain total from that same de-duplicated app-plus-executable merge,
  so the summary cannot disagree with the Toolchain workspace or scriptable
  report. The execution coordinator now passes its shared cancellation token
  into post-action verification callbacks; updater verification uses the
  cancellable provider-review path for its fresh evidence, and uninstall
  verification checks cancellation around its fresh catalog snapshot before
  any receipt commit. The follow-on cancellation slice makes the same
  caller-owned token cover apply-time provider discovery, serial queue refresh,
  manager execution, post-action verification, and the underlying installed-
  software inventory/tool-version probes; a cancelled path fails closed before
  it can publish an action receipt. The interactive performance contract now
  includes versioned PTY startup and refresh-request operations, and the TUI
  exposes an explicit refreshing state so request responsiveness is measurable
  even while the replacement inventory worker is still running.
  The follow-on state-parity slice keeps the text and Ratatui renderers aligned
  for loading, unavailable, empty, and blocked states with visible semantic
  labels, and records those states in the acceptance tests without adding a
  second authority path. The current typed UI contract additionally rejects
  incomplete route sets, generation drift, duplicate record/action/module
  ownership, common host-path forms, and ambiguous route focus; `doctor --help`
  and `config --help` now expose consistent scriptable guidance. Refresh now
  publishes an explicit `refreshing local snapshot` model before the new
  evidence worker runs, preserving the final-artifact performance contract.
- **CLI version:** `0.1.0`.
- **Release posture:** blocked; schema-1 release evidence cannot authorize a
  release.
- **Current writes:** explicit user-local Unix/Windows-guarded store scaffolding,
  a working macOS/Linux/Windows-pre-alpha manager-update executor, and one
  narrow exact-manager uninstall apply boundary exist, with Windows runtime
  evidence still absent. Broad uninstall, recursive cleanup, broad
  quarantine/restore, production module lifecycle execution, and third-party
  execution remain unavailable; the exact-file leftovers and runtime-cache
  quarantine lanes, exact-record restore, developer-only signed module staging,
  and exact-manager uninstall are bounded, pre-alpha write paths.

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
The shared manager challenge-context slice is `7ce8932`.
The developer-only signed module staging slice is `1e782c8`.
The staged-receipt status and TUI count slice is `26c1b44`.
The staged-status evidence refresh is `fb95b45`.
The staged transaction-evidence cross-check slice is `f9e28cb`.
The staged-review TUI warning slice is `b818e54`.
The developer-only installed-inactive promotion slice is `0fe633f`.
The developer-only first-party inventory process invocation slice is
`8f9f3c7`.
The effective configuration review slice is `c6118ad`.
The exact manager-native uninstall execution slice is `1faffa4`.
The persisted inactive lifecycle-state slice is `1186c3d`.
The inactive install-receipt authority slice is `4f90dd5`.
The bounded Flatpak inventory slice is `27b4adf`.
The bounded Flatpak updater parser slice is `6f64415`.
The quiet task-first TUI slice is `d82c60f`.
The bounded Snap updater parser slice is `5285173`.
The portable provider-locale binding slice is `dabc8ec`.
The bounded Zypper XML updater parser slice is `899b2a5`.
The shared Toolchain/TUI executable-evidence slice is `3961200`.
The Home/Toolchain parity slice is `4a8960d`.
The post-action cancellation propagation slice is `7bb5b44`.
The documentation and WinGet-boundary follow-up is `476954d`.
The cancellation-aware inventory and tool-probe slice is `081d92d`.
The apply-time updater discovery cancellation slice is `f53c0e6`.
The cancellable dashboard loading and stale-generation slice is `3930a51`.
The versioned PTY TUI performance slice is `5d0b8f7`.
The current exact-head release evidence refresh is bound to
`5d0b8f728803f02f5c2a674603a8a9224310f1ff`.
Local
`main` and
`origin/main` matched after publication. The source validation baseline passes
`cargo fmt --all -- --check`, `cargo test --workspace --locked`, the full
`cargo test --workspace --locked --all-features` suite, strict all-features
workspace Clippy, Windows MSVC and Linux GNU cross-target `cargo check`, and
`git diff --check`. The prior universal2 package was
`target/release-package-universal2-476954d/runtime-zero-0.1.0-universal2-apple-darwin.zip`.
It has binary SHA-256
`cb6531d4668442e574f7fb482dba15b3f065c5a5c1557b5110c8725749e050d6`, ZIP
SHA-256
`40db7964cdb5930ce4dc09f0e8877f705288b3acca5939e411152fea49e16104`, and
3,994,722 bytes across 8 verified members. The embedded artifact manifest,
SBOM, and third-party notices have SHA-256 values
`8b419840a057b964eaea61a3739a7b31278c1ba0ae80b5293a3ebf09a5de8ecb`,
`5d3e50b53c9d8eec0c5fe64dda2bbd09cb1cf24c0a68d22a114b0b72540d0a71`, and
`378e8e7c8437aa16e51453354e49827e338b7398299c650d28aff6e8a67dcc6c`; their
embedded sizes are 980, 162,322, and 291,176 bytes respectively. The package
verifier passed, and `file`/`lipo` confirmed arm64 plus x86_64 Mach-O slices.
Both slices passed four PTY terminal smoke cases and ten-sample final-artifact
performance evidence. The terminal evidence IDs are
`terminal:universal2-apple-darwin-arm64-cb6531d46684` and
`terminal:universal2-apple-darwin-x86_64-cb6531d46684`; the performance IDs are
`perf:universal2-apple-darwin-arm64-cb6531d46684` and
`perf:universal2-apple-darwin-x86_64-cb6531d46684`. Both slices passed
read-only `doctor`, `scan --dry-run`, `toolchain`, and `config` reviews;
doctor reported 6 passing and 4 blocked policy checks, while scan reported 9
sources, 325 tools, 273 apps, 895 services, and 22 warnings. Toolchain reported 17
records, including 10 observed-only executable records. The configuration
digest is `b4d57157ae30be77f81a293bd49ddc2f939168377b20b9d9bb16a4ea1e40258f`.
The artifact remains unsigned and unnotarized until an owner-led
signing/notarization lane exists; it is not a public release.

The subsequent exact-head universal2 package is
`target/release-package-universal2-3930a51/runtime-zero-0.1.0-universal2-apple-darwin.zip`.
The package verifier passed for source commit
`3930a515693b960c7dc712f5d42febacee3787d1`, target `universal2-apple-darwin`,
and 8 members. Its ZIP SHA-256 is
`c72160f19b9b331c41965aebe8a61a1c266693e5a51acb855327e2826b9b1897` and its
size is 4,004,579 bytes; the universal binary SHA-256 is
`a8d4c3a691177d56546f62d7fc4b8866ed634ded146f0fdd9dfd270ee410acce`. The
embedded artifact manifest, SBOM, and third-party notices have SHA-256 values
`6ec7780e272c23b8a3136aa3ed3aff39666d5134a9a0ac730dd32466451f6c47`,
`d03ecbe1f784e0ed2c4e991baac62ce0ee5f8c50f6cf3ecad05dbbafd94d0e29`, and
`d9210f67237eb56d9e90d848a2f5f4dcbc846985454726140ba4b1cf2b5143cc`; their
embedded sizes are 980, 162,322, and 291,176 bytes. `file` and `lipo`
confirmed arm64 plus x86_64 Mach-O slices. Both slices passed four PTY smoke
cases and ten-sample final-artifact performance evidence with terminal IDs
`terminal:universal2-apple-darwin-arm64-627f497f6435`,
`terminal:universal2-apple-darwin-x86_64-9542dfdffdc8`, and performance IDs
`perf:universal2-apple-darwin-arm64-627f497f6435` and
`perf:universal2-apple-darwin-x86_64-9542dfdffdc8`. Both slices passed
read-only `doctor`, `scan --dry-run`, `toolchain`, and `config` reviews;
doctor reported 6 passing and 4 blocked policy checks; scan reported 9 sources,
325 tools, 273 apps, 895 services, and 22 warnings; Toolchain reported 17 tools including
10 observed-only records; and configuration was valid but non-authorizing.
The artifact remains unsigned and unnotarized and is not a public release.

The current exact-head universal2 package is
`target/release-package-universal2-5d0b8f7/runtime-zero-0.1.0-universal2-apple-darwin.zip`.
The package verifier passed for source commit
`5d0b8f728803f02f5c2a674603a8a9224310f1ff`, target `universal2-apple-darwin`,
and 8 members. Its ZIP SHA-256 is
`c7a3a156ef223248116036e2a98d7522e611e54c91c770ebd182946354ca34e9` and its
size is 4,004,540 bytes; the universal binary SHA-256 is
`166046a6c7bd5280309ff83980509cfb018b538155e54d6e3496a9a34fea7a36`. The
embedded artifact manifest, SBOM, and third-party notices have SHA-256 values
`242f25825a2d67ee1183e55712eb1c5d3f0f36bea18c69092477ec02ab133e2f`,
`306ed93b318fd8539609c37ab606789648c473423a0f61dfd2f4e0dcc43f10e1`, and
`9ab4dfbe47d494c17e8021ffc5400b7933ab8cd66e0b277c07170513915ce317`; their
embedded sizes are 980, 162,322, and 291,176 bytes. `file` and `lipo`
confirmed arm64 plus x86_64 Mach-O slices. Both slices passed four PTY smoke
cases and ten-sample schema-3 final-artifact performance evidence, including
TUI startup and refresh-request operations. The terminal evidence IDs are
`terminal:universal2-apple-darwin-arm64-086d38d21759` and
`terminal:universal2-apple-darwin-x86_64-03a02a5043e4`; the performance IDs are
`perf:universal2-apple-darwin-arm64-086d38d21759` and
`perf:universal2-apple-darwin-x86_64-03a02a5043e4`. Both slices passed
read-only `doctor`, `scan --dry-run`, `toolchain`, and `config` reviews;
the artifact remains unsigned and unnotarized and is not a public release.

A current docs-head unsigned macOS DMG was also built from source commit
`1693654e6140866d8ba9348188f55925f61ce560` at
`target/release-dmg-universal2-1693654/runtime-zero-0.1.0-universal2-apple-darwin.dmg`.
Its SHA-256 is
`da069d18ef2e0f85b2291175f7b33c7896bf10c0970e8dc098878f5fb82b5c10` and its
size is 4,036,312 bytes. The mounted DMG manifest had SHA-256
`51ed98d4e92e81a361509d29445b3b78541483adc638c10c4860cd990ca9fe77`, bound
the source portable ZIP SHA-256
`127cfb52e52c06f1c42173a140bbb9c4546277bdbacb28dc71fc6df829f8dd15`, and
declared `container_reproducible: false`, `signature_posture: unsigned`, and
`notarized: false`. `hdiutil` verified the image checksums; the mounted
contents exposed the exact nine-file DMG contract, and its `rz0 --version`,
`doctor`, and `scan --dry-run` checks passed before clean detachment. This is
local unsigned packaging evidence, not publication, signing, notarization, or
clean-host acceptance.

The release-tooling follow-ups are now at `0c89b7029e01e76cc9f8c2e92dd535e0aa502853`:
the DMG verifier rejects symlinked inputs before resolution and binds mounted
SBOM/third-party-notice bytes to the artifact manifest's exact path, digest,
and size. The exact DMG verification was rerun successfully after both
hardening changes; no packaged binary bytes changed.

The latest clean-head unsigned universal2 package is
`target/release-package-universal2-c5429f6/runtime-zero-0.1.0-universal2-apple-darwin.zip`,
from source commit `c5429f6660481824cf9adc5597904c8e4054063c`. Its package
verifier passed for 8 members with ZIP SHA-256
`e6573113299bbf0de9bc64ae25829010ad72f7ce15a142ad9807eb711d6713a8` and
universal binary SHA-256
`166046a6c7bd5280309ff83980509cfb018b538155e54d6e3496a9a34fea7a36`.
Both ARM64 and Rosetta slices passed four PTY smoke cases and schema-3
performance evidence. The latest TUI startup/refresh-request p95 values were
6,673/703 microseconds on ARM64 and 13,364/947 microseconds on x86_64; the
evidence IDs are
`terminal:universal2-apple-darwin-arm64-166046a6c7bd`,
`terminal:universal2-apple-darwin-x86_64-166046a6c7bd`,
`perf:universal2-apple-darwin-arm64-166046a6c7bd`, and
`perf:universal2-apple-darwin-x86_64-166046a6c7bd`. Extracted package binaries
passed version, doctor, dry-run scan, Toolchain, and configuration
reviews on both slices.

The matching unsigned DMG is
`target/release-dmg-universal2-c5429f6/runtime-zero-0.1.0-universal2-apple-darwin.dmg`.
Its SHA-256 is
`37bedffd06624ddf4da80061015e2f26f66e4761991742bd600b7e8fcf36df1b` and its
size is 4,036,312 bytes. The independent mounted verifier passed the exact
9-member contract, read-only smoke, checksum, source-ZIP, content, license,
SBOM, and notice bindings with `writes_attempted: false` and
`release_authorized: false`. The DMG builder now keeps its commit-scoped
staging under `target/` instead of the system temporary directory. Signing,
notarization, clean-host acceptance, target-native runtime, and public release
remain blocked.

The later secure-fs-head artifact is bound to source commit
`a7db57472584c675f5235bbadc6d43d229fd8ab1`. Its universal2 ZIP is
`target/release-package-universal2-a7db574/runtime-zero-0.1.0-universal2-apple-darwin.zip`
with package-verifier decision `pass`, 8 members, and ZIP SHA-256
`cc4d7549f9b3423aa77656feb837cadcdc4e807fa5af4f17440fa38c84c947d2`. The
matching unsigned DMG is
`target/release-dmg-universal2-a7db574/runtime-zero-0.1.0-universal2-apple-darwin.dmg`
with SHA-256
`32cb95e2401b5b56aa5577e2f52087ebe1901f34ed297a4c195dcfbc30175163`; the
independent verifier passed its exact 9-member mounted contract with
`writes_attempted: false` and `release_authorized: false`. Both slices passed
PTY smoke, package command smoke, and schema-3 performance; TUI startup and
refresh-request p95 were 6,747/747 microseconds on ARM64 and 13,339/1,026
microseconds on x86_64. This remains local unsigned evidence, not a public
release.

The latest exact-head TUI parity slice is `887dbcccfc20f54096d0ed872ffafe662cee4ab9`.
Both Rust renderers now preserve explicit loading, unavailable, empty, and
blocked states; the acceptance guide marks workspace-size, state, and
plain/color semantic automation complete. Human terminal/accessibility review
and the end-to-end provider-review path remain open.

The exact-head unsigned universal2 package is
`target/release-package-universal2-887dbcc/runtime-zero-0.1.0-universal2-apple-darwin.zip`.
Its package verifier passed the exact eight-member contract with ZIP SHA-256
`7b10914531edde11d5b3c9f624536fc19ee4b819bcec5c0c3acfbfc6fdd054c7` and
binary SHA-256
`6c2f2f156feee068eb8d35028531c6babc868b28a7ef80751d8c83a76944e5a2`.
The matching unsigned DMG is
`target/release-dmg-universal2-887dbcc/runtime-zero-0.1.0-universal2-apple-darwin.dmg`
with SHA-256
`55a99d6f8243a82e0d797e2624e9c918b2f93e09dbdbfc102116246cd0c2b466`.
Its mounted verifier passed the exact nine-member contract, read-only smoke,
and source-ZIP/content/SBOM/notice bindings with `writes_attempted: false`
and `release_authorized: false`. ARM64 and Rosetta x86_64 each passed four
final-artifact PTY cases, package command smoke, and ten-sample schema-3
performance. TUI startup/refresh p95 was 7,015/741 microseconds on ARM64 and
13,226/976 microseconds under Rosetta. This is exact local unsigned evidence,
not signing, target-native runtime, accessibility, or public-release approval.

The current exact-head universal2 package was rebuilt from source commit
`eb7178a724063130b02d886879293009fc81dd47` at
`target/release-package-universal2-eb7178a-v2/runtime-zero-0.1.0-universal2-apple-darwin.zip`.
The independent package verifier passed the exact eight-member contract. Its
ZIP SHA-256 is
`4ec85afeb5b48f284050147eeb5995d59714ca8d530b60a35348778c900f8af2` and its
universal binary SHA-256 is
`5db1e311a7bb58884f5f696bea64c7f6ab9d3d7746b0c6ab574675069afac2bc`.
`file` and `lipo` confirmed arm64 plus x86_64 Mach-O slices. Both slices
passed four PTY terminal cases, extracted-package command smoke, and
ten-sample schema-3 performance evidence. Performance evidence IDs are
`perf:universal2-apple-darwin-arm64-5db1e311a7bb` and
`perf:universal2-apple-darwin-x86_64-5db1e311a7bb`. TUI startup/refresh p95
was 6,848/261 microseconds on arm64 and 16,019/567 microseconds under
Rosetta. This artifact remains unsigned and unnotarized and is not a public
release.

The current release decision remains blocked. On this source head, `doctor`
reports 6 passing and 4 blocked policy checks; `scan --dry-run` is read-only
with no writes; and the final artifact passed the bounded terminal and
performance contracts above. Earlier recovery evidence inspected 11 valid
transaction journals, found 0 invalid journals, and classified all 11 as
action-required under the conservative assessment; one bounded warning reported
persistent writer-lock markers without claiming active ownership. Linux release
linking was re-attempted with the
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
recovery beyond local journal finalization, module trust/lifecycle, broad
uninstall/cleanup execution, accessibility,
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
- dry-run uninstall findings, sealed manager action plans, and one narrow
  exact-manager apply lane with shared receipts and fresh verification;
- Linux opened-executable identity-to-spawn binding, bounded process-group
  teardown, caller cancellation, exact updater write evidence, canonical
  external-effect receipts, and read-only recovery assessment;
- provider-native macOS/Linux updater execution for Homebrew formulae/casks,
  Apple Software Update, npm prefixes, pip, RubyGems, rustup, uv, Cargo,
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
  Windows runtime, reparse/ACL, and capability-isolation proof remain
  incomplete. The interactive updater now has a Rust-native Windows console
  control bridge for Ctrl+C/Ctrl+Break, including duplicate-registration
  protection and clean handler teardown; target-native event delivery remains
  unverified here;
- Windows secure-fs creation now requests the required handle rights and applies
  a protected current-user/SYSTEM/Administrators DACL plus current-user owner
  before the existing strict privacy verifier runs. This is source and MSVC
  compile evidence only; real client/server filesystem, inherited-ACL, reparse,
  owner, and flush acceptance remains open.
- Unix process groups are containment aids, not syscall/filesystem/network/
  privilege sandboxes, and a hostile child may attempt session escape;
- cancellation now covers confirmed updater discovery, serial refresh,
  manager execution, post-action verification, and installed-software
  inventory/tool probes; remaining production process/write hosts and
  platform-runtime proof are still open;
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
pip, RubyGems, crates.io Cargo installs, Warp's standalone
CLI store, and declared Electron/Squirrel releases. Deno is explicitly
delegated to its Homebrew formula because the installed binary lacks native
self-upgrade support. MacPorts, Mac App Store, and Hermes were reported as
missing on this host; 12 Sparkle bundles were observed-only because Sparkle's
public tooling does not provide a generic external app-update command.

Live smoke work committed OMP and Pi/npm-prefix effects through the
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
rz0 uninstall apply <installed-software-id> --executable <absolute-path> --accept-no-rollback [--challenge-issued-unix-seconds <seconds>] [--confirm <phrase>] [--format text|json]
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
| TUI `c` selected reviewable action | Provider metadata plus manager network write where required | Manager plus private journal/receipt writes | Direct shared foundation process-host flow with exact TUI confirmation; Windows remains pre-alpha |
| `updates --all-providers` | Providers may read remote metadata after acknowledgement | No product write | Provider-driven bounded review across installed managers, language environments, self-updaters, and declared app metadata; missing, observed-only, and unsupported sources remain warnings |
| updater fixture/captured output | No | No | Implemented review/planning |
| `updates --recovery-status` | No | No | Implemented deterministic evidence assessment only |
| `updates --recovery-complete` | Runtime.zero private journal only | No manager rerun | Implemented fresh receipt-bound local finalization; no rollback or automatic mutation |
| `updates --apply` | Explicit read/write acknowledgement; not OS-isolated | Manager plus private journal/receipt writes | Working macOS/Linux/Windows pre-alpha lane with receipts; Windows runtime/ACL/reparse proof remains open |
| uninstall plan | No | No | Shared finding and optional sealed action plan; no execution from the plan form |
| uninstall apply | Manager command may alter package state; network intent is not requested by this boundary | Manager plus private journal/receipt writes | One exact manager-owned, destructive-confirmation lane with executable identity binding, explicit no-rollback acknowledgement, cancellation, and fresh inventory verification; broad/dependent/rollback support remains open |
| module validation/install planning | No | No | Implemented planning only; optional complete-file-set review rejects undeclared files |
| `modules install --developer-trial` | Local package read plus explicit runtime.zero-owned write | Staged bytes and transaction/stage receipts; optional test-key-only installed-inactive registry/receipt with `--developer-promote` | Implemented developer-only staging/promotion; no activation, production trust, or public distribution |
| `modules invoke --developer-trial` | Local promoted package read plus bounded Rust child process | No registry/lifecycle write; path-redacted inventory response and executable binding evidence | Implemented only for promoted first-party.inventory; no activation, lifecycle receipt, sandbox, third-party execution, or production authority |
| `modules trust verify` | No | No | Test-key-only exact package review; no production trust root or lifecycle authority |
| `modules status` | No | No | Path-redacted registry/receipt plus developer-staging view; reports staged, installed-inactive, or degraded, never active |
| store plan/status | No | No | Implemented read-only inspection |
| `store init --yes` | No | Runtime.zero-owned user-local scaffold | Unix and guarded Windows owner/DACL path; runtime acceptance remains open |
| broad uninstall/cleanup/module lifecycle execution | — | — | Not implemented; only the narrow exact-manager uninstall apply boundary exists |

Network flags express intent; they do not create an OS network sandbox. No
command uploads a report or installs an elevation helper.

## Interactive TUI

Bare `rz0` opens the full-screen TUI only for an interactive stdin/stdout pair
without recognized automation variables. Explicit subcommands never enter the
TUI. Terminal guards restore raw mode, cursor, mouse capture, and alternate
screen on normal exit and panic unwinding; ordinary broken pipes are clean exits
for scriptable output.

The five stable destinations are Overview, Explore, Review, Activity, and
Modules. The TUI renders a loading shell before the local snapshot worker
finishes, and `r` is the only explicit retry. Startup and refresh workers carry
a cancellation token; `q` cancels in-flight work, `r` cancels the previous
generation, and stale worker results cannot overwrite a newer snapshot.
Controls include `r` refresh, `u` explicit provider review, `c` foundation
confirmation for a selected reviewable action, `/` search, arrows/`j`/`k`,
Home/End, Tab/Shift+Tab, Enter, mouse wheel, `h`/`?`, Esc, and `q`.

The TUI has no second mutation implementation: `c` delegates to the same exact
plan, confirmation, identity-bound process, transaction, receipt, verification,
and recovery evidence path as the CLI. It cannot invent module lifecycle or
recovery authority.

## Inventory and identity

The installed core embeds the bounded `modules/inventory` library. Current
metadata sources include:

- process PATH and allowlisted executable discovery;
- persisted Windows User/Machine PATH, standard uninstall registry views, and
  service/driver registry metadata;
- macOS application bundles and bundle IDs, Homebrew Cellar/Caskroom roots,
  MacPorts roots, Apple Installer receipt plists, and launchd plist labels;
- Linux XDG desktop entries/desktop IDs, direct dpkg status, pacman local and
  Flatpak `active/metadata` records, and systemd unit-file labels;
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

Coverage remains incomplete: RPM/DNF, AppImage, Nix, language managers,
containers, browser extensions, live service status/dependencies,
MSIX/AppX/Winget/Chocolatey/Scoop, and many persistence/driver details await an
explicit 1.0 scope and target-native proof. Snap now has bounded table parsing,
but its manager/runtime and target-native proof remain open.

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
Apple Software Update, npm prefixes, language tools,
crates.io Cargo installs, Warp's standalone CLI store, and declared
Electron/Squirrel releases. Winget remains fail-closed because its documented
list surface is human-readable; Zypper has a strict XML package-row parser;
Flatpak has a strict JSON, ref/commit-bound parser and Snap has a strict
five-column table parser. Findings, action plans, and queue plans
remain non-authorizing until the apply lane consumes an exact confirmation.

Live discovery observes the exact manager artifact and seals its SHA-256, size,
and platform identity into each plan. Replacement invalidates plan/confirmation
identity. Winget remains explicitly unavailable until a stable, locale-safe
machine interface is proven.

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
`user_requested` cancellation. On Windows, Ctrl+C and Ctrl+Break are converted
through a static console-control callback and a bounded polling bridge into the
same typed token. The host terminates/reaps the process group and publishes
recovery-required evidence where possible. It does not reverse an external
effect already performed.

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

`rz0 uninstall apply <id> --executable <path>` is the first narrow execution
boundary. It re-derives the finding and single-action plan, requires a fresh
short-lived destructive challenge plus `--accept-no-rollback`, binds the exact
manager executable, uses the shared cancellation/transaction/external-effect
receipt path, and requires fresh installed-software verification. The manager
may use fixed non-interactive elevation when the action contract requires it;
runtime.zero never collects credentials or opens an interactive helper. The
plan form remains non-authorizing, and the apply form does not provide
dependent-package review, recursive cleanup, user-bundle handling, rollback,
or recovery automation. The exact leftovers lane remains a separate
single-file quarantine path.

## Module catalog and lifecycle

All seven first-party manifests remain `planned`:

| Family | Current implementation | Major missing work |
| --- | --- | --- |
| Inventory/environment | Embedded read-only collector plus development binary | Full source/platform parity and signed lifecycle |
| Updater | Provider-driven plans plus working macOS/Linux core executor | Windows isolation, rollback/recovery, manager/runtime matrix, release proof |
| Uninstall | Shared synthetic/live findings, dry-run manager plans, and one narrow exact-manager apply boundary | Dependent/shared-component review, complete manager/platform adapters, rollback/quarantine/recovery, and broad cleanup |
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
not current commands and must wait for production lifecycle execution,
foundation-owned registry authority, trust, configuration, receipts, recovery,
and module-host execution.

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
| `performance-contract` | Eleven read-only command budgets, including PTY TUI startup/refresh request timing | Target-native evidence and refresh-completion budgets |
| `release-contract` | Target × module × stage ledger | RC freeze/evidence population |

## Validation baseline

Current source validation for
`e3f4bd40c8b36acf6f68e1a0979e2d5dd05b84d2`:

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
- the Ratatui buffer matrix passed for all five destinations at 58x16, 80x24,
  118x30, and 160x50 in plain and color modes;
- final-artifact `doctor`, `scan --dry-run`, `cache --dry-run`, and
  `recovery --dry-run` JSON reviews passed privacy checks; recovery exposed
  bounded journal decisions and review warnings without raw paths, while cache
  exposed the policy, age, scan, and active-use uncertainty fields;
- `git diff --check` passed;
- the Toolchain review passed focused contract tests, live CLI JSON inspection,
  help/completion coverage, and remains read-only with no provider invocation or
  state writes;
- the TUI Toolchain workspace passed the full Ratatui/render/state/dashboard
  suites with shared executable evidence and no second action authority; dense
  rows preserve the selected-context pane;
- the universal2 package was rebuilt from the exact source baseline above at
  `target/release-package-universal2-3961200`, and the package verifier passed
  with the archive SHA-256
  `f8d5623b273c181669afa2b28756550bc18d6738b68171336683674a5fe25313`;
- the trust fixture review returned a valid package/signature result while
  retaining `test_key_only: true`, `execution_authorized: false`, and
  `writes_attempted: false`;
- the universal2 ZIP was independently verified from this exact package source
  head; both arm64 and x86_64 slices passed four PTY cases and ten-sample
final-artifact performance evidence. Toolchain reported 17 records with 10 observed-only
  executable records. The package verifier reported 8 members and pass; the
  artifact remains unsigned and unnotarized.

Historical earlier validation also recorded completion-source parity, Bash/Zsh
syntax, PowerShell parsing, `mandoc`, Markdown-link checks, module-manifest
checks, secret/private-path scans, and diff hygiene. Fish and ShellCheck were
unavailable in that lane, so Fish had static coverage rather than a native
parser run.

`cargo-audit` and `cargo-deny` were unavailable and were not auto-installed.
Locked metadata still resolved 150 packages (31 workspace and 119 external), and
native target-filtered release metadata covered 119 reachable packages (96
external). Older continuation records also cover `cargo check --workspace`,
doctor, scan, and development-Mac updater smoke evidence. No live uninstall
mutation, Cloudflare/site mutation, release publication, or production release
action was run in those lanes.

## Known limitations

### Product and UX

- No stable 1.0 CLI/JSON/API compatibility guarantee exists.
- TUI update actions now enter the shared direct confirmation/execution flow;
  uninstall in the TUI and cleanup actions remain reviews rather than execution
  flows because the narrow apply boundary is currently CLI-only.
- No broad uninstall, cleanup, integrity remediation, or module
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
5. Expand and prove the narrow manager-native uninstall boundary through
   manager/platform adapters, dependency review, confirmation, durable receipts,
   rollback or quarantine-first recovery, and fault testing.
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
