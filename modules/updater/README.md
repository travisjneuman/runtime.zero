# Updater module

The updater is a first-party domain module. The standalone binary maps
caller-supplied installed/manager evidence into the shared finding contract and
can bind update candidates to the foundation action-plan contract. The core
owns the manager execution lane so the module cannot invent its own process,
confirmation, or transaction stack.

Each provider adapter is explicit and bounded. Provider-specific output is
parsed only by the matching adapter; malformed, truncated, invalid, or
oversized captures fail closed rather than becoming guessed update candidates.
The resulting action identity includes the exact manager, target, executable
identity, arguments, and evidence digest, so the shared foundation can enforce
planning, confirmation, execution, receipt, and fresh verification without a
second provider authority.

The standalone binary accepts bounded JSON on standard input:

```bash
rz0-updater --format json < updater-finding-input.json
rz0-updater --plan --format json < updater-finding-input.json
rz0-updater --plan --queue --format json < updater-finding-input.json
```

The core can also parse a locally captured manager response without invoking the
manager:

```bash
rz0 updates --dry-run --manager homebrew-formula \
  --manager-output /tmp/homebrew-outdated.json \
  --executable /opt/homebrew/bin/brew --plan --queue --format json

# Provider-driven live review across the current host
rz0 updates --dry-run --all-providers --allow-network-read --plan --queue --format json

rz0 updates --recovery-status --transaction <exact-transaction-id>

# After reviewing the challenge emitted by --apply without --confirm:
rz0 updates --apply --probe --manager homebrew-formula \
  --executable /opt/homebrew/bin/brew --allow-network-read \
  --allow-network-write --action <exact-action-id> \
  --accept-no-rollback --challenge-issued-unix-seconds <issued> \
  --confirm '<exact-phrase>'
```

The action plan and serial queue remain `dry_run: true` planning artifacts with
`writes_attempted: false` and `product_execution_authorized: false`. Queue items
are ordered, individually identified, and pause on failure, drift, cancellation,
or recovery requirements. Missing installed/manager evidence or an exact
absolute manager executable remains blocked. The core exposes explicit
`rz0 updates --apply`, targeted aggregate `--action`, and interactive serial
`--all` lanes. They require live evidence, an allowlisted manager artifact whose
exact identity is sealed into the plan, explicit network-write approval, an
initialized private store, an exact short-lived confirmation, and a
manual-recovery acknowledgement when rollback is not proven. Linux launches
through the held executable descriptor; macOS uses path identity/digest
revalidation; Windows uses the shared Rust process host's pre-start Job Object
and explicit inherited-handle list. The direct process is cancellable, bounded, journaled with exact
write intent and outcome, backed by a canonical external-effect receipt, and
followed by fresh verification. Elevated managers use the fixed `/usr/bin/sudo`
wrapper with `sudo -n`; npm updates receive an isolated temporary cache and
preserve the discovered user/runtime PATH.

The module includes bounded, locale-reviewed parser slices for Homebrew JSON,
APT, DNF, Pacman, MacPorts, Flatpak JSON, Mac App Store `mas` JSON lines, Apple
`softwareupdate --list`, global npm prefixes, pip JSON, RubyGems, `rustup`,
`uv tool`, Grok, Hermes, and oh-my-pi. Homebrew cask review uses documented
greedy mode so latest/auto-updating casks are not silently omitted. The
provider resolver also executes native update lanes for crates.io Cargo
installs and Warp's standalone signed CLI store. On
macOS it inspects Electron/Squirrel application release metadata when the
bundle declares a GitHub provider and identifies Sparkle bundles that must
remain on their signed in-app channel.

This means a single review can catch tools such as Codex, Pi, GSD, Kilo, and
other npm-owned CLIs when their actual npm prefix is present; it also checks
native Grok and OMP channels and reports Hermes when installed. It does not
invent an update command for a direct installer, a private vendor service, an
unknown bundle, or an app whose channel is only available inside its UI.
Winget parsing currently fails closed because its documented list surface is
still human-readable. Zypper uses `--xmlout list-updates` with `--no-refresh`
and accepts only exact package rows from the `update-list`; malformed XML,
patches, missing identity, and attribute drift fail closed. Snap uses the exact
five-column `snap refresh --list` table under the updater's forced `C` locale;
header, row shape, and bounded field validation all fail closed on drift.
Flatpak uses the explicit `remote-ls --updates --app --json` column contract
under the updater's forced `C` locale and binds each candidate to its exact
app/architecture/branch ref plus the remote commit. The human-readable version
field is validated for bounded shape but is not used as authority because
Flatpak versions can be source metadata rather than the immutable update
identity. A probe specification is not production runtime evidence.
`--all-providers` is provider-driven and bounded: missing, delegated,
observed-only, and unsupported sources remain explicit in the report.

## Apply-lane limitations

The core apply lane is pre-alpha rather than a supported module lifecycle:

- Linux binds and revalidates a direct native ELF manager through the held
  `/proc/self/fd`; script/interpreter chains remain blocked; macOS revalidates
  the visible path and digest immediately before spawn; Windows lacks complete
  handle-to-image/Job Object containment;
- network read/write flags record explicit intent but do not create an OS
  network sandbox;
- Unix process groups, bounded output, and SIGINT cancellation are not a
  capability sandbox and do not reverse an external effect;
- native rollback, full boundary-by-boundary cancellation, exact recovery
  completion, and platform power-loss proof are missing;
- read-only recovery status can reconcile journal/receipt evidence but cannot
  mutate, retry, repair, or finish a commit;
- live provider review and dry-run planning have been exercised on the
  development Mac for the discovered channels; provider apply claims remain
  subject to receipt, fresh-verification, and recovery evidence. That evidence
  would still not substitute for the broader platform/release matrix or native
  rollback proof.

Third-party module execution, uninstall/cleanup mutation, module lifecycle, and
release support remain separate foundation gates. See
[`../../docs/project-status-and-resumption.md`](../../docs/project-status-and-resumption.md)
and [`../../docs/completion-checklist.md`](../../docs/completion-checklist.md).
