# runtime.zero

**System Management Toolkit**  
Command: `rz0`

`runtime.zero` is a Rust-first, terminal-native foundation for safe system management. The core stays intentionally small: it owns the CLI, policy, output contracts, and module registry primitives while substantial capabilities ship as explicit modules instead of being bundled by default.

> Status: pre-alpha Phase 1 bootstrap. This repository is public early so the design and safety model are visible from the start. Destructive modules are intentionally not implemented yet.

## The promise

`runtime.zero` is designed to feel like a dark terminal control surface while behaving like a careful system steward:

- report first;
- dry-run first;
- quarantine before delete;
- manager-native uninstall before file cleanup;
- no surprise installs;
- no credential/session cleanup without explicit approval;
- no persistence, malware behavior, evasion, or account actions.

## Supported command surface today

```bash
rz0 --version
rz0 doctor
rz0 modules
rz0 modules --format json
rz0 scan --dry-run
```

Current commands are read-only or dry-run stubs. They exist to prove the binary, brand metadata, test harness, documentation foundation, and module contract surface.

## Core vs modules

The installed `rz0` foundation is not meant to contain every feature. It should remain useful with zero optional modules installed:

- `core.cli` handles command routing and output.
- `core.policy` defines shared safety metadata and future mutation gates.
- `core.registry` lists core primitives and explicitly installed modules.

First-party feature modules are planned as separate install/use choices. A full bundle may exist later as a convenience distribution, but it should not redefine the core. Third-party modules require a hardened trust model before support is added.

## Platform target

The initial support target is modern Windows, macOS, and mainstream Linux distributions.

- Windows 10 / 11 and Windows Server 2016+
- current macOS on Apple Silicon and Intel where Rust supports it
- mainstream Linux x86_64 / aarch64
- best-effort expansion for older or niche systems over time

The long-term goal is broad terminal compatibility. The public compatibility promise will stay honest: old OS releases and niche distributions will be treated as best-effort until specifically tested.

## Development

```bash
cargo test
cargo run -- --version
cargo run -- doctor
cargo run -- modules
cargo run -- modules --format json
cargo run -- scan --dry-run
```

## Architecture

The project is intentionally modular:

- Rust CLI core for command parsing, action planning, policy, logs, JSON output, and quarantine/restore.
- Platform adapters for Windows, macOS, and Linux.
- Optional modules for update, uninstall, leftover scan, cleaner, security/integrity checks, and future ideas.

See [`docs/architecture.md`](docs/architecture.md) and [`docs/module-system.md`](docs/module-system.md).

## Repository hygiene

The project root is intentionally kept small and conventional. Source belongs in
`src/`, product docs in `docs/`, site material in `site/`, and future tests,
scripts, fixtures, or assets should live in clearly named subfolders. Durable
planning and session artifacts belong in `_meta.notes`, not as loose root files.

## Website

A minimal static site lives in [`site/`](site/). It is prepared for a future `rz0.neuman.dev` or `runtimezero.neuman.dev` deployment. No Cloudflare or GitHub Actions deployment automation is configured in Phase 1.

## License

Apache-2.0. See [`LICENSE`](LICENSE).
