# Dependency and Validation Audit — 2026-07-29

This snapshot records the manual foundation/inventory audit at the current
pre-alpha source stage. It is evidence, not a promise of future security support
or a substitute for release-time review.

## Toolchain and checks

Validated with Rust/Cargo 1.96.0:

- `cargo fmt --check`;
- `cargo test --workspace`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- `cargo check --workspace --target x86_64-pc-windows-msvc`;
- Windows-target clippy with the same `-D warnings` posture;
- Linux-target workspace check and Clippy with warnings denied;
- `cargo run -- doctor` and `cargo run -- scan --dry-run`;
- fixture and live-redacted inventory JSON parsing/assertions;
- opt-in macOS application-bundle smoke with redacted paths;
- synthetic Linux XDG desktop-entry precedence/parser tests;
- test-key signature, immutable OS-temp staging, quarantine/restore failure, and
  no-execution process-protocol fixtures;
- opt-in version-probe smoke with redacted output;
- pseudo-terminal TUI navigation/help/quit/alternate-screen restoration smoke;
- static site checks for unsafe JavaScript primitives and missing local links;
- `cargo-deny 0.20.2` advisory, license, ban, and source-policy checks using the
  committed `deny.toml`.

The Windows target check compiles the read-only `winreg` adapters but is not a
Windows runtime test. A real Windows smoke remains required.

## RustSec advisory scan

`cargo-audit 0.22.2` loaded 1,173 RustSec advisories and scanned the 121 entries
reported from `Cargo.lock`. It reported no known vulnerabilities.

This result is time-bound to 2026-07-29. No recurring workflow was added;
release candidates must run a fresh advisory scan.

## License metadata

`cargo metadata --locked` resolved six local workspace packages and 115
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

The separate module-trust crate uses `ed25519-dalek` 3.0 with default features
disabled for strict, local test-key signature verification. It does not expose a
runtime signer, generate keys, fetch trust metadata, or join the core runtime
dependency graph. The module-protocol crate adds only Serde and remains a
fixture validator with no process-spawn dependency.

The core Ratatui graph contains two `hashbrown` versions through Ratatui's
internal `kasuari`/`lru` graph. The audit found one crossterm backend version and
no duplicate terminal-control stack.

## Remaining release gates

- Real Windows registry/app/version-timeout/redaction runtime tests.
- Real Linux desktop-entry/application and terminal runtime smoke; broader
  macOS terminal compatibility beyond the current inventory smoke.
- Artifact-level license/notice and reproducibility checks.
- Signed provenance/key lifecycle and revocation design implementation.
- Capability enforcement, production transaction/receipt durability, and
  process/platform isolation; immutable staging/quarantine/restore currently
  exist only as OS-temp integration-test simulations.
- Separately approved release, package publishing, bootstrap, deployment, and
  recurring automation.
