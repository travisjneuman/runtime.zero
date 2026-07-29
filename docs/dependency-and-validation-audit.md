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
- macOS Apple Silicon native evidence plus Intel target check/Clippy;
- `cargo run -- doctor` and `cargo run -- scan --dry-run`;
- fixture and live-redacted inventory JSON parsing/assertions;
- opt-in macOS application-bundle smoke with redacted paths;
- synthetic Linux XDG desktop-entry precedence/parser tests;
- test-key signature, immutable OS-temp staging, quarantine/restore failure,
  no-execution module-protocol fixtures, canonical blocked production-execution
  assessments, and native test-child framing/output/timeout failure cases;
- opt-in version-probe smoke with redacted output;
- pseudo-terminal TUI navigation/help/quit/alternate-screen restoration smoke;
- static site checks for unsafe JavaScript primitives and missing local links;
- `cargo-deny 0.20.2` advisory, license, ban, and source-policy checks using the
  committed `deny.toml`.

The default workspace suite passed 197 tests. The all-features suite passed 206,
including the shared capability/error/resource semantics and five exact release-
acceptance cross-product tests, seven transaction-chain/recovery
unit tests, four guarded immutable-snapshot recovery tests, five opened-artifact identity tests, six production-execution gate
tests, and nine native macOS test-child
transport cases. The transport cases prove
fail-closed refusal of an observed inheritable descriptor and Unix process-group
teardown of a sleeping descendant with inherited pipes. The Windows-target lane
also compiles private kill-on-close Job Object assignment, a two-process
ceiling, descendant creation, and timeout job termination. Windows x86/x86-64/ARM64, Linux x86-64/ARM64, and macOS Intel all-feature target
checks are not runtime evidence. Ordinary Rust Windows targets require Windows
10/Server 2016. The Tier-3 legacy checks compile crates and a custom standard
library but do not link an EXE or prove any Windows 7/8/Server runtime.

## RustSec advisory scan

`cargo-audit 0.22.2` loaded 1,173 RustSec advisories and scanned the 127 entries
reported from `Cargo.lock`. It reported no known vulnerabilities.

This result is time-bound to 2026-07-29. No recurring workflow was added;
release candidates must run a fresh advisory scan.

## License metadata

`cargo metadata --locked` resolved twelve local workspace packages and 115
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

The check passes with one warning: Ratatui's internal graph currently resolves
`hashbrown` 0.16.1 and 0.17.1. The duplicate is visible rather than silently
excepted and does not duplicate the terminal backend. No recurring workflow was
added; this remains a manual/release-candidate command.

## Dependency shape

The first-party inventory module depends on the small
`rz0-inventory-contract` model crate rather than the core CLI/TUI package. Its
normal cross-platform dependencies are Serde, serde_json, and time; Windows adds `winreg`. This avoids pulling Ratatui/Crossterm into
`rz0-inventory`.

The release-contract crate adds no new external package and bounds the canonical
target × seven-module × 12-stage evidence ledger to 256 targets/21,504 cells
while remaining unable to authorize release. The resource-contract crate adds no
new external package and centralizes typed
process limits plus artifact/document/inventory/probe ceilings reused across
foundation and inventory packages. The error-contract crate adds no new external
package and replaces free-form
module-protocol error codes with stable typed Serde values plus conservative
privacy/retry classifiers. The transaction-contract crate adds no new external
package: it reuses Serde and SHA-256 for a bounded domain-separated event chain,
state validation, and non-authorizing recovery assessment. Its library performs
no I/O. The separate module-trust
crate uses `ed25519-dalek` 3.0 with default features
disabled for strict, local test-key signature verification. It does not expose a
runtime signer, generate keys, fetch trust metadata, or join the core runtime
dependency graph. The capability-contract crate adds only Serde and centralizes
the vocabulary/classifiers reused by core manifests, process protocols, and
action plans; it grants no authority. The artifact-identity crate uses SHA-256
plus the existing Windows system bindings only on Windows to obtain stable
opened-handle file identity; it has no spawn or installer API. The
module-protocol crate uses Serde for its default fixture validator. Its
non-default test-child feature adds Serde JSON framing; SHA-256 is a dev
dependency, and Windows test builds use the already-resolved `windows-sys`
Job Object APIs. Process spawn code exists only in integration-test support, not
the library, core, CLI, or TUI.

The core Ratatui graph contains two `hashbrown` versions through Ratatui's
internal `kasuari`/`lru` graph. The audit found one crossterm backend version and
no duplicate terminal-control stack.

## Remaining release gates

- Real Windows registry/app/version-timeout/redaction runtime tests.
- Real Linux desktop-entry/application and terminal runtime smoke; broader
  macOS terminal compatibility beyond the current inventory smoke.
- Artifact-level license/notice and reproducibility checks.
- Signed provenance/key lifecycle and revocation design implementation.
- Verified-handle-to-execution binding, descriptor/handle-inheritance race
  closure, Windows suspended-create and real Job Object runtime proof,
  production capability enforcement, and platform sandbox runtime proof;
  current process evidence executes only the Cargo test helper.
- Production transaction/receipt durability; immutable staging/quarantine/
  restore currently exist only as OS-temp integration-test simulations.
- Separately approved release, package publishing, bootstrap, deployment, and
  recurring automation.
