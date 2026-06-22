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
rz0 --no-tui
rz0 doctor
rz0 modules
rz0 modules --format json
rz0 modules validate <manifest.json>
rz0 modules --from <directory> --format json
rz0 modules install --dry-run <package-dir-or-manifest>
rz0 store plan
rz0 store plan --format json
rz0 store status
rz0 store status --format json
rz0 scan --dry-run
```

Bare `rz0` opens a minimal read-only TUI dashboard in an interactive terminal.
Use `rz0 --no-tui` for the scriptable text dashboard, or `rz0 --json` for a
machine-readable foundation dashboard. `rz0 <subcommand>` remains scriptable and
never opens the TUI.

Current commands are read-only or dry-run stubs. They exist to prove the binary,
brand metadata, test harness, documentation foundation, TUI shell, and module
contract surface.

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
rz0 modules install --dry-run path/to/module-package
```

This is local, read-only validation and planning only. The install planner
reports proposed locations and state changes, but it does not write files,
install, update, fetch, trust, enable, or run modules.

The dry-run planner also reports future local store and CLI/TUI routing
contract metadata in JSON output. These fields describe where future state would
live and why explicit subcommands remain scriptable; they do not create files.

The same future store/routing contract can be inspected independently of module
install planning:

```bash
rz0 store plan
rz0 store plan --format json
rz0 store status
rz0 store status --format json
```

These commands are read-only. `store plan` reports the platform-specific
user-local store roots, registry and transaction paths, example
receipt/quarantine/rollback paths, forbidden path classes, and current CLI/TUI
launch-routing interpretation. `store status` checks whether those future paths
already exist and also parses an existing `installed-modules.json` registry if
present. It reports absent, empty, valid, invalid, or unreadable registry state,
schema version, installed module count, duplicate IDs, malformed records, and
unsafe path references. When a valid registry record references an existing
receipt, `store status` validates that receipt shape and cross-checks module
ID/version and store-relative paths. It still does not create directories,
write state, repair anything, trust modules, execute code, or imply modules are
active.

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
cargo run --
cargo run -- --no-tui
cargo run -- --json
cargo run -- --version
cargo run -- doctor
cargo run -- modules
cargo run -- modules --format json
cargo run -- modules validate path/to/rz0-module.json
cargo run -- modules install --dry-run path/to/module-package
cargo run -- store plan
cargo run -- store plan --format json
cargo run -- store status
cargo run -- store status --format json
cargo run -- scan --dry-run
```

## Local install for development

To make `rz0` available from a normal PowerShell terminal on a development
machine, use the local-only install script:

```powershell
.\scripts\install-local.ps1 -DryRun -AddToPath
.\scripts\install-local.ps1 -AddToPath
```

The script builds the checked-out binary, copies it to
`%USERPROFILE%\.local\bin\rz0.exe`, writes a local install marker, and adds that
directory to the **user** PATH only when `-AddToPath` is supplied. Open a new
PowerShell terminal after installing before expecting `rz0` to resolve outside
the repository.

Rollback is also local and explicit:

```powershell
.\scripts\uninstall-local.ps1 -DryRun -RemovePath
.\scripts\uninstall-local.ps1 -RemovePath
```

See [`docs/local-install.md`](docs/local-install.md) for the safety boundaries
and options, including how rollback treats pre-existing user PATH entries. This
is not a public release, installer, package manager, bootstrap command, or
install-from-internet flow.

## Architecture

The project is intentionally modular:

- Rust CLI core for command parsing, action planning, policy, logs, JSON output, and quarantine/restore.
- Platform adapters for Windows, macOS, and Linux.
- Optional modules for update, uninstall, leftover scan, cleaner, security/integrity checks, and future ideas.
- Minimal read-only TUI shell for local review; subcommands remain the stable
  automation/script surface.

See [`docs/architecture.md`](docs/architecture.md),
[`docs/module-system.md`](docs/module-system.md), and
[`docs/manifest-validation.md`](docs/manifest-validation.md). See
[`docs/store-and-routing-contract.md`](docs/store-and-routing-contract.md) for
the read-only future module store and CLI/TUI launch-routing contract.

## Brand system

The canonical public brand guide is [`BRAND.md`](BRAND.md).

Current direction: **Dossier Navy / Burnished Brass** — blackened navy,
graphite panels, bone-white type, burnished-brass operational accents, muted
blue-gray metadata, and red only for danger/error/destructive states.

Owner-provided candidate assets live under [`assets/brand/`](assets/brand/).
They are candidates, not final locked identity assets.

## Repository hygiene

The project root is intentionally kept small and conventional. Source belongs in
`src/`, product docs in `docs/`, site material in `site/`, brand assets in
`assets/brand/`, and future tests, scripts, fixtures, or other assets should
live in clearly named subfolders. Durable planning and session artifacts belong
in `_meta.notes`, not as loose root files.

## Website

The first static landing page is live at [`https://rz0.neuman.dev`](https://rz0.neuman.dev) and its source lives in [`site/`](site/). It is deployed through the connected Cloudflare Worker project `runtime-zero` using `site/` as the static output directory.

This first version is dependency-free and public-safe, but the visual direction is still provisional. Website visual editing is currently paused until stronger reference examples are reviewed. Future site work should align to [`BRAND.md`](BRAND.md), avoid red as a brand accent, keep claims honest, avoid unsafe direct-run commands, and preserve the static deployment unless a framework migration is separately approved.

## License

Apache-2.0. See [`LICENSE`](LICENSE).
