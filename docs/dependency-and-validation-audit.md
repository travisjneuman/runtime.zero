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
- `cargo run -- doctor` and `cargo run -- scan --dry-run`;
- fixture and live-redacted inventory JSON parsing/assertions;
- opt-in version-probe smoke with redacted output;
- pseudo-terminal TUI navigation/help/quit/alternate-screen restoration smoke;
- static site checks for unsafe JavaScript primitives and missing local links.

The Windows target check compiles the read-only `winreg` adapters but is not a
Windows runtime test. A real Windows smoke remains required.

## RustSec advisory scan

`cargo-audit 0.22.2` loaded 1,173 RustSec advisories and scanned the 106 entries
reported from `Cargo.lock`. It reported no known vulnerabilities.

This result is time-bound to 2026-07-29. No recurring workflow was added;
release candidates must run a fresh advisory scan.

## License metadata

`cargo metadata --locked` resolved four local workspace packages and 102
external packages. Every external package declared license metadata. Observed
license expressions were combinations of:

- MIT and/or Apache-2.0;
- Apache-2.0 with LLVM exception;
- Unicode-3.0;
- BSL-1.0 as an alternative;
- CC0-1.0 as an alternative;
- Unlicense as an alternative;
- Zlib.

This is metadata inventory, not legal advice or a completed redistribution
notice audit. Before publishing binaries, verify selected license alternatives,
required notices/source offers, artifact contents, and final dependency graph.

## Dependency shape

The first-party inventory module depends on the small
`rz0-inventory-contract` model crate rather than the core CLI/TUI package. Its
normal cross-platform dependencies are Serde, serde_json, and time; Windows adds
`winreg`. This avoids pulling Ratatui/Crossterm into `rz0-inventory`.

The core Ratatui graph contains two `hashbrown` versions through Ratatui's
internal `kasuari`/`lru` graph. The audit found one crossterm backend version and
no duplicate terminal-control stack.

## Remaining release gates

- Real Windows registry/app/version-timeout/redaction runtime tests.
- Linux and macOS terminal/inventory compatibility matrix beyond the current
  macOS smoke.
- Artifact-level license/notice and reproducibility checks.
- Signed provenance/key lifecycle and revocation design implementation.
- Capability enforcement, immutable staging, transaction/rollback simulation,
  and process isolation.
- Separately approved release, package publishing, bootstrap, deployment, and
  recurring automation.
