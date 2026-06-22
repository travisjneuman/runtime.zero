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
rz0 modules validate <manifest.json>
rz0 modules --from <directory> --format json
rz0 scan --dry-run
```

Current commands are read-only or dry-run stubs. They exist to prove the binary, brand metadata, test harness, documentation foundation, and module contract surface.

## Core vs modules

The installed `rz0` foundation is not meant to contain every feature. It should remain useful with zero optional modules installed:

- `core.cli` handles command routing and output.
- `core.policy` defines shared safety metadata and future mutation gates.
- `core.registry` lists core primitives and explicitly installed modules.

First-party feature modules are planned as separate install/use choices. A full bundle may exist later as a convenience distribution, but it should not redefine the core. Third-party modules require a hardened trust model before support is added.

The foundation can validate local module manifests without executing module
code. Installed manifests must also pass local SHA-256 integrity checks for
explicitly listed package files:

```bash
rz0 modules validate path/to/rz0-module.json
rz0 modules --from path/to/installed-modules --format json
```

This is local, read-only validation only. It does not install, update, fetch,
trust, enable, or run modules.

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
cargo run -- modules validate path/to/rz0-module.json
cargo run -- scan --dry-run
```

## Architecture

The project is intentionally modular:

- Rust CLI core for command parsing, action planning, policy, logs, JSON output, and quarantine/restore.
- Platform adapters for Windows, macOS, and Linux.
- Optional modules for update, uninstall, leftover scan, cleaner, security/integrity checks, and future ideas.

See [`docs/architecture.md`](docs/architecture.md),
[`docs/module-system.md`](docs/module-system.md), and
[`docs/manifest-validation.md`](docs/manifest-validation.md).

## Repository hygiene

The project root is intentionally kept small and conventional. Source belongs in
`src/`, product docs in `docs/`, site material in `site/`, and future tests,
scripts, fixtures, or assets should live in clearly named subfolders. Durable
planning and session artifacts belong in `_meta.notes`, not as loose root files.

## Website

The first static landing page is live at [`https://rz0.neuman.dev`](https://rz0.neuman.dev) and its source lives in [`site/`](site/). It is deployed through the connected Cloudflare Worker project `runtime-zero` using `site/` as the static output directory.

This first version is dependency-free and public-safe, but the visual direction is still provisional. The next website pass should refine the terminal-noir / Mr. Robot-inspired feel while keeping claims honest, avoiding unsafe direct-run commands, and preserving the static deployment unless a framework migration is separately approved.

## License

Apache-2.0. See [`LICENSE`](LICENSE).
