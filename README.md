# runtime.zero

**System Management Toolkit**  
Command: `rz0`

`runtime.zero` is a Rust-first, terminal-native foundation for safe system management: scan, update, uninstall, inspect leftovers, quarantine, restore, and eventually plug in specialized modules without turning the tool into one giant unsafe cleanup script.

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
rz0 scan --dry-run
```

Phase 1 commands are read-only or dry-run stubs. They exist to prove the binary, brand metadata, test harness, and documentation foundation.

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
cargo run -- scan --dry-run
```

## Architecture

The project is intentionally modular:

- Rust CLI core for command parsing, action planning, policy, logs, JSON output, and quarantine/restore.
- Platform adapters for Windows, macOS, and Linux.
- Modules for update, uninstall, leftover scan, cleaner, security/integrity checks, and future ideas.

See [`docs/architecture.md`](docs/architecture.md) and [`docs/module-system.md`](docs/module-system.md).

## Website

A minimal static site lives in [`site/`](site/). It is prepared for a future `rz0.neuman.dev` or `runtimezero.neuman.dev` deployment. No Cloudflare or GitHub Actions deployment automation is configured in Phase 1.

## License

Apache-2.0. See [`LICENSE`](LICENSE).
