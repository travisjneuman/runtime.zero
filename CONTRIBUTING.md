# Contributing

`runtime.zero` is public from the beginning, but it is still pre-alpha and its
system-management safety model is under active development. Issues, design
feedback, documentation corrections, platform evidence, accessibility reviews,
and security analysis are welcome. Large code contributions may be deferred
until the associated contract, threat model, and maintainer review path are
stable.

By participating, you agree to keep examples synthetic and to follow
[`SAFETY.md`](SAFETY.md), [`SECURITY.md`](SECURITY.md), and the relevant topic
contract under [`docs/`](docs/documentation-index.md).

## Before proposing a change

1. Read [`docs/project-status-and-resumption.md`](docs/project-status-and-resumption.md)
   for current behavior and known gaps.
2. Use [`docs/documentation-index.md`](docs/documentation-index.md) to find the
   owning contract.
3. Check the roadmap and production matrix so a fixture, parser, or compile
   result is not mistaken for a complete platform feature.
4. Open an issue or discussion before broad architecture, dependency, module,
   trust, mutation, packaging, release, website/deployment, or automation work.
5. Keep each change narrow enough that its safety and rollback impact can be
   reviewed independently.

## Contribution rules

- Do not submit code that performs destructive cleanup, stealth persistence,
  credential/session access, evasion, unauthorized account actions, or broad
  unknown-data mutation.
- Preserve report-first, dry-run-first, quarantine-first, manager-native-first,
  and exact-confirmation-first behavior.
- Do not weaken protected-data classes, path validation, resource ceilings,
  privacy defaults, executable identity, process containment, transaction
  ordering, or fail-closed unsupported behavior to make a test pass.
- Keep substantial domain behavior outside the core. Shared policy, trust,
  capability, process, filesystem, transaction, privacy, error, configuration,
  diagnostics, and release behavior belongs in foundation crates.
- Do not add a second software list for update/uninstall/cleanup actions; attach
  options to the canonical installed-software identity view.
- Do not add shell/PATH execution for security-sensitive commands. Manager
  execution must use an exact reviewed adapter and foundation process host.
- Do not add production module execution, signing keys, feeds, installers,
  package publication, release automation, Cloudflare changes, recurring work,
  telemetry, or external service writes without maintainer approval.
- Keep credentials, private paths, identities, customer/employer data, personal
  software inventories, and host-specific output out of source, fixtures,
  issues, screenshots, and documentation.
- Filesystem-writing tests must remain confined to marked direct children of the
  operating-system temporary root and must revalidate their cleanup boundary.
- Do not claim runtime support from cross-compilation, fixtures, Rosetta, a test
  helper, or another operating system.

## Repository layout

- `src/` — installed CLI/TUI foundation and built-in read adapters;
- `crates/` — shared policy and contract libraries;
- `modules/` — separately built first-party domain packages;
- `tests/` — foundation integration tests and public-safe fixtures;
- `docs/` — product, architecture, contract, platform, and release docs;
- `scripts/` — local validation/packaging helpers;
- `site/` — connected static public site; changes may deploy and require a
  separately reviewed lane;
- `assets/brand/` — candidate assets with provenance;
- repository root — only conventional public Rust project files.

Do not add loose planning notes, logs, screenshots, generated reports, or
machine-local evidence to the repository root.

## Development setup

The workspace currently targets Rust 1.96 and edition 2024. Use locked
resolution for validation:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --workspace --locked --all-features
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo run --locked -- doctor --format json
cargo run --locked -- scan --dry-run --format json
cargo run --locked -- apps --format json
cargo run --locked -- monitor --format json
cargo run --locked -- report --format json
cargo run --locked -- completions bash
```

For dependency or release-candidate changes, also run the committed
`cargo deny check` policy and a fresh advisory scan when the separately installed
tools are available. Do not auto-install those tools from a contribution script.

Process-transport changes must run:

```bash
cargo test -p rz0-module-protocol --locked --all-features
```

The explicit feature may execute only the committed Cargo test helper under its
guarded temporary root. Updater changes must cover fixture/captured/live-plan
boundaries without applying a real update to a normal development host.

Run cross-target, packaging, final-artifact, PTY, performance, fault, and
platform-runtime lanes only when they are relevant and available. State exactly
which checks were not run.

## Documentation changes

Documentation is part of the safety surface:

- update CLI help, README, safety language, narrow contracts, current-status
  guide, and module README together when behavior changes;
- preserve dated evidence with its source commit and toolchain;
- distinguish `implemented`, `fixture-tested`, `compile-checked`,
  `runtime-verified`, `release-supported`, and `planned`;
- use relative links and verify them;
- keep JSON field names and command examples synchronized with source tests;
- never turn old private `_meta.notes` host evidence into a public support claim.

## Pull request or patch checklist

- [ ] Scope and non-goals are stated.
- [ ] Threat, privacy, write, network, privilege, and recovery effects are stated.
- [ ] Shared foundation ownership is preserved.
- [ ] Inputs, outputs, and resource limits are bounded.
- [ ] Unknown/unsupported states fail closed with useful output.
- [ ] Valid, missing, duplicate, malformed, oversized, adversarial, and partial-
      failure cases are covered where relevant.
- [ ] Text, JSON, and TUI claims agree and remain understandable without color.
- [ ] Targeted tests, full relevant tests, formatting, and strict Clippy pass.
- [ ] Dependency/license/advisory checks are included when applicable.
- [ ] Documentation and public claims are current.
- [ ] No secrets, private paths, identities, generated build output, or unrelated
      changes are included.

## Licensing

The repository currently uses Apache-2.0. By submitting a contribution, you
must have the right to contribute it under that license. Do not add third-party
artwork, fonts, code, generated assets, or data without compatible provenance
and required notices. Future premium or commercial modules may use separate
licenses, but none is implied by the current public workspace packages.
