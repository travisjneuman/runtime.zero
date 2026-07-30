# Dependency and Validation Audit — 2026-07-29

This snapshot records the manual foundation/inventory audit at the current
pre-alpha source stage. It is evidence, not a promise of future security support
or a substitute for release-time review.

## Toolchain and checks

Validated with Rust/Cargo 1.96.0:

- `cargo fmt --check`;
- `cargo test --workspace`;
- `cargo test -p rz0-module-protocol --all-features` for the explicit test-child
  transport;
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
- `cargo check --workspace --target x86_64-pc-windows-msvc`;
- Windows modern-baseline x86, x86-64, and ARM64 target checks/Clippy with the
  same `-D warnings` posture;
- pinned `nightly-2026-07-29` build-std workspace checks for the Tier-3
  Windows-7-baseline x86 and x86-64 MSVC targets;
- Linux GNU x86-64 and ARM64 target checks/Clippy with warnings denied;
- macOS Apple Silicon native evidence, Intel target checks, Intel final-artifact
  Rosetta execution, and deterministic universal2 ZIP execution through both
  slices;
- `cargo run -- doctor` and `cargo run -- scan --dry-run`;
- fixture and live-redacted inventory JSON parsing/assertions;
- opt-in macOS application-bundle smoke with redacted paths;
- synthetic Linux XDG desktop-entry precedence/parser tests;
- test-key signature, immutable OS-temp staging, quarantine/restore failure,
  no-execution module-protocol fixtures, canonical blocked production-execution
  assessments, and native test-child framing/output/timeout failure cases;
- opt-in version-probe smoke with redacted output;
- source-test pseudo-terminal TUI navigation/help/quit/alternate-screen smoke;
- final universal artifact PTY smoke through ARM64 and Rosetta x86-64 across
  `xterm-256color`, resized compact `xterm`, wide `screen`, and minimum `vt100`;
- static site checks for unsafe JavaScript primitives and missing local links;
- deterministic target-filtered SPDX 2.3 and deduplicated third-party license/
  notice generation bound to the final native binary;
- bounded read-only final-artifact performance sampling for version, diagnostics,
  core scan, and dashboard text/JSON paths;
- adversarial checksum/entry/traversal/symlink DMG-content preparation tests;
- `cargo-deny 0.20.2` advisory, license, ban, and source-policy checks using the
  committed `deny.toml`.

The default workspace suite passed 271 tests. The all-features suite passed 281.
Coverage includes shared capability families plus exact manifest/protocol/action
grant validation, error/resource/validation/confirmation,
four privacy-redaction tests, four fail-closed configuration tests, four strict
config-digest-bound diagnostic tests, four final-artifact performance contract
tests, two process-host
capture/descriptor tests, action-plan digests, opened-directory adversarial
tests, canonical registries, release ledgers, transaction/recovery chains,
durable writers, commit receipts, default and fault-enabled commit coordination,
store initialization, opened-artifact identity, fail-closed native executable
binding, production-execution assessments, and nine native macOS guarded test-
child cases. The transport cases prove fail-closed refusal of an observed
inheritable descriptor and Unix process-group teardown of a sleeping descendant
with inherited pipes. Linux/Windows test-host builds hold the verified executable
lease through spawn. The Windows-target lane also compiles private kill-on-close
Job Object assignment, a two-process ceiling, strict owner/DACL inspection,
descendant creation, and timeout job termination. Windows x86/x86-64/ARM64,
Linux x86-64/ARM64, and macOS Intel all-feature target
checks are not runtime evidence. Ordinary Rust Windows targets require Windows
10/Server 2016. The Tier-3 legacy checks compile crates and a custom standard
library but do not link an EXE or prove any Windows 7/8/Server runtime.

## RustSec advisory scan

`cargo-audit 0.22.2` loaded 1,173 RustSec advisories and scanned the 138 entries
reported from `Cargo.lock`. It reported no known vulnerabilities.

This result is time-bound to 2026-07-29. No recurring workflow was added;
release candidates must run a fresh advisory scan.

## License metadata

`cargo metadata --locked` resolved 23 local workspace packages and 115
external packages. Every external package declared license metadata. Observed
license expressions were combinations of:

- MIT and/or Apache-2.0;
- Apache-2.0 with LLVM exception;
- Unicode-3.0;
- BSL-1.0 as an alternative;
- CC0-1.0 as an alternative;
- Unlicense as an alternative;
- Zlib;
- BSD-3-Clause for the direct Ed25519 verification implementation.

This is metadata inventory, not legal advice or a completed redistribution
notice audit. Before publishing binaries, verify selected license alternatives,
required notices/source offers, artifact contents, and final dependency graph.

## Manual dependency policy

`deny.toml` evaluates the macOS, Linux GNU, and Windows MSVC graphs with all
features. It denies wildcard registry dependencies, unknown registries, unknown
Git sources, advisories, and licenses outside the current permissive allowlist.
Workspace path dependencies carry explicit local versions so they do not act as
wildcard requirements.

The check passes with two warnings: Ratatui's internal graph currently resolves
`hashbrown` 0.16.1 and 0.17.1, and the alternative `BSD-3-Clause` allowance was
not selected in the evaluated target graph. The duplicate is visible rather
than silently excepted and does not duplicate the terminal backend. No recurring workflow was
added; this remains a manual/release-candidate command.

## Dependency shape

The first-party inventory module depends on the small
`rz0-inventory-contract` model crate rather than the core CLI/TUI package. Its
normal cross-platform dependencies are Serde, serde_json, and time; Windows adds `winreg`. This avoids pulling Ratatui/Crossterm into
`rz0-inventory`.

The secure-fs crate adds no newly resolved external package; it reuses libc and
windows-sys for Unix runtime-tested and Windows compile-checked held-directory
operations, locks, atomic publication, Unix ownership/mode checks, and Windows
exact-owner/bounded-DACL inspection. Windows ACL behavior remains compile-only
and does not enable store initialization.
The confirmation-contract crate adds no newly resolved external package and
binds exact plan/dry-run/write-set/state digests to short-lived interactive
responses and single-use consumption evidence without execution authority. The cancellation-
contract crate adds no external package beyond the shared error vocabulary and
uses one `Arc<AtomicU8>` for first-writer-wins reasons and overflow-safe monotonic
deadlines; guarded process timeout polling consumes it. The module-lifecycle
crate reuses shared validation plus SHA-256 to own dry-run transition/gate policy
for all eight lifecycle operations without execution authority. The
registry-contract crate adds no newly resolved external package and owns the
bounded canonical installed-state model, exact paths/order, serialization, and
digests consumed by core reporting and transactions. The validation-contract crate adds no external package and provides allocation-free
canonical grammar consumed by foundation ID/version/hash/path parsers. The
release-contract crate adds no new external package and bounds the canonical
target × seven-module × 12-stage evidence ledger to 256 targets/21,504 cells
while remaining unable to authorize release. The resource-contract crate adds no
new external package and centralizes typed
process limits plus artifact/document/inventory/probe/redaction/diagnostic
ceilings reused across foundation and inventory packages. The error-contract crate adds no new external
package and replaces free-form
module-protocol error codes with stable typed Serde values plus conservative
privacy/retry classifiers. The transaction-contract crate adds no newly resolved external package: it
reuses foundation action, confirmation, registry, resource, validation, and
secure-filesystem crates for a bounded domain-separated event chain, immutable
snapshot publication/recovery, durable single-use consumption, commit receipts,
rollback evidence, atomic registry-last coordination, and non-authorizing commit
recovery assessment. The separate module-trust
crate uses `ed25519-dalek` 3.0 with default features
disabled for strict, local test-key signature verification. It does not expose a
runtime signer, generate keys, fetch trust metadata, or join the core runtime
dependency graph. The capability-contract crate adds only Serde and centralizes
the vocabulary/classifiers reused by core manifests, process protocols, and
action plans; it grants no authority. The artifact-identity crate uses SHA-256
plus the existing Windows system bindings only on Windows to obtain stable
opened-handle file identity. Its borrow-scoped executable-binding API uses Linux
`/proc/self/fd`, Windows deny-write/delete handle retention, and explicit
fail-closed unsupported behavior on macOS; it grants no execution authority and
has no installer API. The
privacy-contract crate adds no new external package and uses shared SHA-256 for
bounded domain-separated report-local placeholders without retaining raw values.
The configuration-contract crate adds no new external package and owns canonical
immutable offline/default-deny schema-1 settings and their digest. The diagnostics-contract crate adds no new external package and owns the strict
private text/JSON doctor model. The performance-contract crate adds no new external package and owns canonical
final-artifact command ceilings plus strict non-authorizing evidence. The
process-host crate adds no new external package, centralizes bounded capture plus Unix descriptor auditing, and places
process-group/Job Object helper containment behind `test-support`; it exposes no
production runner. The module-protocol crate uses Serde for its default fixture
validator. Its non-default test-child feature adds Serde JSON framing and
consumes the process-host test foundation. SHA-256 remains a dev dependency.
Process spawn code exists only in integration-test support, not the library,
core, CLI, or TUI.

The core Ratatui graph contains two `hashbrown` versions through Ratatui's
internal `kasuari`/`lru` graph. The audit found one crossterm backend version and
no duplicate terminal-control stack.

## Remaining release gates

- Real Windows registry/app/version-timeout/redaction runtime tests.
- Real Linux desktop-entry/application and terminal runtime smoke; broader
  macOS terminal compatibility beyond current inventory and universal/Rosetta
  artifact smoke; Rosetta is not Intel hardware or older-macOS proof.
- Final legal review of generated artifact-level license/notice evidence and
  reproducibility checks on every target; native target-filtered generation is
  deterministic local evidence only.
- Signed provenance/key lifecycle and revocation design implementation.
- Linux/Windows executable-binding runtime proof, a reviewed macOS handle-to-
  spawn primitive, descriptor/handle-inheritance race closure, Windows suspended-
  create and real Job Object runtime proof,
  production capability enforcement, and platform sandbox runtime proof;
  current process evidence executes only the Cargo test helper.
- Windows owner/DACL and directory-flush evidence, production cancellation
  propagation, rollback execution, and real process/power-loss recovery;
  deterministic coordinator boundary injection and exact registry-completion
  recovery are local evidence, while staging/quarantine/restore remain OS-temp
  integration simulations.
- Separately approved release, package publishing, bootstrap, deployment, and
  recurring automation.
