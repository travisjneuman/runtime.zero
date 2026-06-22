# AGENTS.md — runtime.zero

## Scope

These instructions apply to the entire `runtime.zero` repository.

## Project intent

`runtime.zero` is a public, Rust-first, terminal-native system management
toolkit. The launch command is currently `rz0`. It must stay safety-led:
report-first, dry-run-first, quarantine-first, and explicit-confirmation-first.

## Root cleanliness

Keep the repository root clean and conventional. Root-level files should be
limited to items that belong at the top of a public Rust CLI repository:

- `README.md`
- `LICENSE`
- `SAFETY.md`
- `SECURITY.md`
- `CONTRIBUTING.md`
- `BRAND.md`
- `AGENTS.md`
- Rust manifests/locks such as `Cargo.toml` and `Cargo.lock`
- root config files that tooling genuinely requires

Place everything else in the narrowest appropriate folder:

- product/design docs in `docs/`
- Rust source in `src/`
- website/source landing page material in `site/`
- tests in `tests/` when integration tests are added
- scripts/helpers in `scripts/` when needed
- fixtures/examples/assets in clearly named subfolders
- brand source/candidate assets in `assets/brand/`

Do not add loose root-level planning notes, scratch files, screenshots, logs,
exports, prompts, reports, or temporary artifacts. Durable planning and session
artifacts belong in `E:/Web Development/_meta.notes/Projects/runtime.zero/`.

## Brand rules

`BRAND.md` is the canonical public brand reference. Keep the default visual
direction aligned to Dossier Navy / Burnished Brass: blackened navy, graphite,
bone-white text, burnished brass accents, muted blue-gray metadata, and red only
for danger/error/destructive states. Do not use red, rust, oxblood, crimson, or
copper-red as brand accents. Keep original owner-generated visual assets outside
the repo untouched; copy selected candidates into `assets/brand/` before any
repo/site/README/TUI use.

## Safety rules

- Do not implement destructive cleanup, uninstall, install, update execution,
  persistence, account actions, or credential/session handling without explicit
  current-session approval.
- Any future mutating capability must have a dry-run path, risk classification,
  and rollback/quarantine story before it can be enabled.
- Never auto-delete credentials, OAuth sessions, browser profiles, project
  workspaces, backups, or unknown user data.
- Prefer manager-native uninstall/update mechanisms before direct filesystem
  cleanup.
- Keep examples public-safe: no private hostnames, internal IPs, usernames,
  tokens, secrets, customer/employer data, or Travis-only paths.

## Build and validation

Before handing off code changes, run the smallest relevant checks first:

- `cargo fmt --check`
- `cargo test`
- `cargo run -- doctor`
- `cargo run -- scan --dry-run`

Use heavier checks only when they add value for the change. Do not add GitHub
Actions, release automation, package publishing, Cloudflare deployment, or
other recurring/quota-impacting automation without explicit approval.

## Git

Work on `main` by default. Do not create feature branches unless Travis asks.
Commit and push completed intentional changes with:

`Travis J. Neuman <travis@neuman.dev>`

Never add agent attribution trailers.
