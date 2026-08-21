# User Guide

`runtime.zero` is an active pre-alpha. This guide describes the current source
behavior; it is not a support or production-release promise. Start with
[`SAFETY.md`](../SAFETY.md) and use `rz0 --help` as the exact parser authority.

## Operating model

The normal progression is:

```text
bounded evidence -> finding/review -> dry-run plan -> explicit confirmation
-> one bounded action -> fresh verification -> durable receipt/recovery state
```

Most commands stop before confirmation and action. Evidence, identifiers,
findings, plans, confirmations, executable bindings, receipts, and release
ledgers do not grant broader authority merely because they validate.

Core invariants:

- report first and preserve useful partial source results;
- dry-run before any supported write;
- manager-native action before filesystem removal;
- quarantine before any future direct removal;
- exact, short-lived, single-use confirmation;
- no shell/PATH lookup for security-sensitive manager execution;
- no hidden `sudo`, UAC helper, install, network request, or retry;
- no credentials, sessions, projects, backups, shared data, or unknown data in
  a cleanup action.

## Build and first run

There is no public direct-run installer. From a trusted local checkout:

```bash
cargo build --locked --release
./target/release/rz0 --version
./target/release/rz0 doctor
./target/release/rz0 scan --dry-run
```

See [`local-install.md`](local-install.md) for the current local-only install
helpers. Packaging scripts build local evidence and never publish a release.

Bare `rz0` opens the TUI only when stdin/stdout are terminals and automation is
not detected. Use `rz0 --no-tui` for deterministic text or `rz0 --json` for the
private dashboard summary. Explicit subcommands never launch the TUI.

## Read-only workflows

### Diagnostics

```bash
rz0 doctor
rz0 doctor --format json
```

Diagnostics omit host name, user name, current directory, environment values,
and raw paths. The updater policy check reports platform-specific execution
posture. On macOS, manager apply is available as a pre-alpha path-revalidated
lane; Windows uses pre-start Job Object/handle-list containment but remains
pre-alpha pending runtime and ACL/reparse evidence.

### Installed software and source identity

```bash
rz0 apps
rz0 apps --format json
```

The catalog is path-free. Current records can include source-specific identifiers
such as bundle IDs, manager package IDs, package receipt IDs, desktop IDs, and
Windows product/registry IDs. An identifier shared by multiple sources takes
precedence over display-name heuristics; otherwise name-normalized groups remain
explicitly heuristic and preserve version disagreement.

Current bounded sources include:

- macOS application bundles, Homebrew Cellar/Caskroom metadata, MacPorts metadata
  roots, Apple Installer receipt plists, and launchd labels;
- Linux XDG desktop entries, direct dpkg status and pacman local metadata, and
  systemd unit-file labels;
- Windows persisted PATH, standard uninstall registry views, product codes/key digests, and
  service registry metadata;
- process PATH and allowlisted tool discovery on all three platform families.

### Rust, AI, and developer toolchain

```bash
rz0 toolchain
rz0 toolchain --format json
```

This is a bounded local snapshot of toolchain records and provider posture. It
does not invoke AIUP, Cargo, rustup, npm, or another provider, and it never
installs or updates anything. Provider states remain explicit (`ready`,
`observed-only`, or a later failed/blocked state) so an observed binary is not
mistaken for an executable update authority. The TUI Toolchain workspace uses
the same classification; provider availability review remains the separate
read-only `u`/`rz0 updates --dry-run --all-providers` workflow.

Source identifiers, software names, versions, publishers, and service labels may
be sensitive even when paths are omitted.

### Full bounded inventory

```bash
rz0 scan --dry-run
rz0 scan --dry-run --format json
```

Paths are report-locally redacted by default. `--include-raw-paths` is for local
inspection only and makes the report unsuitable for the support-export privacy
gate.

The inventory records source status independently. `unavailable` or `partial`
means the source did not silently disappear from the result.

### Cache evidence

```bash
rz0 cache --dry-run
rz0 cache --dry-run --format json
rz0 cache --dry-run --fixture tests/fixtures/cache/valid.json --format json
```

Cache review is bounded and read-only. Live mode inspects only known
runtime.zero, Homebrew, npm, pip, and Cargo cache roots; it skips symlinks and
special files and stops at explicit entry/byte ceilings. Findings are
ownership-aware but never authorize cleanup, quarantine, restore, or deletion.
The JSON result contains a `cache_review` envelope and the shared
`classified_finding_report`. User/shared/unknown cache data remains report-only
or blocked. Each live observation also reports a 30-day review-age threshold,
the number/bytes over that threshold, modification-time bounds, scan
completeness, and a conservative lock-marker signal. A missing lock marker is
not proof that the cache is inactive; process/native lock proof is not
available in this bounded adapter. The TUI Diagnostics workspace shows the
same observation and age-threshold status.

The complete ownership, budget, active-use, and exclusion contract is in
[`docs/cache-management.md`](cache-management.md).

An explicitly supplied regular file inside the runtime.zero cache root can use
the separate exact plan/apply lane. It binds the file digest and size, prints a
short-lived challenge, and quarantines only that file after the exact phrase is
re-entered; it never performs recursive cleanup, deletion, elevation, or
network access.

```bash
rz0 cache --dry-run --plan --path /absolute/path/to/runtime-zero-cache-file
rz0 cache --apply --path /absolute/path/to/runtime-zero-cache-file
rz0 cache --apply --path /absolute/path/to/runtime-zero-cache-file \
  --challenge-issued-unix-seconds <issued> --confirm '<exact phrase>'
```

### Leftover evidence

```bash
rz0 leftovers --dry-run
rz0 leftovers --dry-run --format json
rz0 leftovers --dry-run --fixture tests/fixtures/leftovers/valid.json --format json
```

Leftover review is bounded and read-only. Live mode inspects only runtime.zero
module, log, and unreferenced receipt roots; it never scans the home directory,
profile, drive, PATH, package-manager receipts, services, or launch entries.
Receipt ownership is checked only when the installed-module registry is valid;
ambiguous registry state is reported as unavailable. Symlinks and special files
are skipped, entry/byte ceilings are enforced, and metadata evidence is
report-only because it does not prove stale ownership or a safe exact-file
transaction. For an explicitly known regular file inside the runtime.zero
module store, the separate plan/apply lane can bind a digest and size, print a
short-lived challenge, and quarantine only that file after the exact phrase is
re-entered. It never performs recursive cleanup, deletion, elevation, or
network access; broad domain quarantine and restore remain unavailable.

```bash
rz0 leftovers --dry-run --plan --path /absolute/path/to/runtime-zero-module-file
rz0 leftovers --apply --path /absolute/path/to/runtime-zero-module-file
rz0 leftovers --apply --path /absolute/path/to/runtime-zero-module-file \
  --challenge-issued-unix-seconds <issued> --confirm '<exact phrase>'
```

### Exact quarantine restore

Before restoring, inspect the bounded quarantine inventory without exposing
absolute host paths or writing state:

```bash
rz0 recovery --dry-run
rz0 recovery --dry-run --format json
```

The review reports only logical plan/action identity, digest/size evidence,
record validity, payload presence, and whether the narrow restore lane can use
the record. It also inspects bounded immutable transaction journal heads and
reports stable per-transaction state, recovery decision, operator guidance,
invalid-journal count, and incomplete-review warnings without exposing the
private state root. Persistent writer-lock markers are evidence only; their
presence does not prove active ownership. The review is capped, skips unsafe
record entries, and never deletes, restores, repairs, publishes a journal,
completes a transaction, or authorizes rollback.

Restore is intentionally separate from cache and leftovers discovery. It reads
one existing runtime.zero quarantine record, validates the record binding,
recomputes the original cache/module destination, and refuses symlinked,
occupied, or drifted paths. The dry-run is read-only:

```bash
rz0 restore --dry-run --plan-id <exact-quarantine-plan-id>
```

The apply invocation first prints a five-minute challenge. Re-enter its exact
phrase with the issued timestamp to restore only that payload:

```bash
rz0 restore --apply --plan-id <exact-quarantine-plan-id> \
  --challenge-issued-unix-seconds <issued> --confirm '<exact phrase>'
```

Restore uses the durable filesystem-effect journal and receipt path. It never
overwrites an occupied destination, deletes quarantine data recursively,
elevates, fetches network content, or turns a broad cache/leftover review into
an action. Retention and permanent deletion remain separate product gates.

### Integrity evidence

```bash
rz0 integrity --dry-run --fixture tests/fixtures/integrity/valid.json --format json
rz0 integrity --dry-run --path /absolute/path/to/file --sha256 <sha256> --format json
```

Integrity review accepts either bounded caller-supplied fixture evidence or one
absolute regular file plus an expected SHA-256 digest. The exact-file adapter
uses the shared opened-artifact identity check and omits the path from output.
Neither form is a trusted, versioned, revocable runtime baseline. The shared
contract preserves exact digest observations and can mark mismatches high-risk,
but it never claims malware or vulnerability detection and never authorizes
remediation, quarantine, restore, or deletion.

### System monitor

```bash
rz0 monitor
rz0 monitor --format json
```

This is a one-shot local snapshot. Metric depth differs by platform; see
[`system-monitor.md`](system-monitor.md). The command does not install a daemon,
start telemetry, or retain samples.

### Privacy-reviewed local report

```bash
rz0 report
rz0 report --format json
```

This foundation surface combines redacted live inventory with private
diagnostics and emits summary counts/statuses and domain-separated digests. It
omits raw reports, paths, host/user identity, application/service names, process
output, and free-form warnings. `local_export_ready: true` means the local
summary passed the strict contract; `external_sharing_authorized` remains false.
Review even a summary before sharing it.

## Interactive TUI

The task-first TUI has five workspaces: Home, Toolchain, Software, System,
and Diagnostics. It renders a loading shell before the full local inventory
and monitor snapshot completes; an explicit `r` refresh is the only retry.
Important controls:

- `r`: refresh local inventory;
- `u`: scan every discovered provider for availability and potentially read
  network metadata; it never applies an update;
- `U`: compatibility shortcut for Review action on the highlighted installed-
  software or provider row. The TUI
  refreshes exact evidence, shows the manager/target/command and challenge
  phrase, then executes after the phrase is entered and accepted;
- `m`: select the System workspace;
- `/`: search; `f`: filter; `s`: sort;
- arrows or `j`/`k`: move; Home/End: boundaries;
- Tab/Shift+Tab: change focus among navigation, details, and the selected
  context pane; Enter/Space: details;
- `h`/`?`: help; Esc: back; `q`: quit.

The TUI and CLI share the same exact action plan, confirmation, transaction,
identity binding, receipt, and post-update verification path. The TUI is the
primary interactive workflow; the CLI remains the scriptable equivalent and is
the recovery-status escape hatch. On a first TUI update, runtime.zero creates
only its own user-local state scaffold if needed; no manager write starts before
the exact challenge is accepted. See [`tui.md`](tui.md).

## Uninstall review

```bash
rz0 uninstall plan <installed-software-id>
rz0 uninstall plan <id> --executable /opt/homebrew/bin/brew --format json
```

The command now converts live catalog evidence into the shared uninstall finding
contract. Manager-owned records also receive a finding-bound action plan:

- without a sealed exact executable identity, the action is present but
  `blocked`;
- with an allowlisted direct manager executable, the dry-run action may be
  `planned` and includes its SHA-256/size identity;
- protected system software stays blocked;
- user/local bundles remain report-only pending a production root-relative,
  quarantine-first action contract;
- unknown/package-receipt-only ownership remains unsupported or blocked.

A `planned` uninstall action is still non-authorizing. No uninstall process,
file move, quarantine, deletion, dependent-package review, rollback, or restart
is performed.

## Update review and execution boundary

### Review local or captured evidence

```bash
rz0 updates --dry-run --fixture tests/fixtures/updater/evidence.json
rz0 updates --dry-run --fixture tests/fixtures/updater/evidence.json --plan
rz0 updates --dry-run --fixture tests/fixtures/updater/evidence.json --plan --queue
```

A fixture cannot become execution input. Captured manager output can build an
execution-ready plan only when its exact local executable can be observed and
sealed.

### Explicit live availability

```bash
rz0 updates --dry-run --probe \
  --manager homebrew-formula \
  --executable /opt/homebrew/bin/brew \
  --allow-network-read \
  --plan

# Review all provider lanes discovered on this host
rz0 updates --dry-run --all-providers \
  --allow-network-read \
  --plan --queue
```

The executable is opened directly, bounded, hashed, identified, and sealed into
the resulting plan. A second invocation must reproduce the same plan identity;
manager executable replacement invalidates confirmation.

`--all-providers` resolves installed provider owners instead of assuming that a
bundle name identifies its update channel. It probes system managers, global
language/package environments, and known self-updaters when an exact
availability/update adapter is available. On this Mac that includes Homebrew,
Apple Software Update, npm global prefixes, pip, RubyGems, Grok, oh-my-pi, `uv`,
AIUP-managed native tools, crates.io Cargo installs, Warp's standalone CLI, and
declared Electron/Squirrel GitHub release metadata; it also reports Hermes,
MacPorts, Mac App Store, and Sparkle apps when present. This catches npm-owned
CLIs such as Codex, Pi, GSD, and Kilo when their actual prefix is discovered.
Every source is reported as successful, missing, unavailable, delegated, or
observed-only.
Direct installers, private vendor services, unknown bundles, and UI-only
channels remain visible gaps until a reviewed owner adapter exists. This is
provider-driven bounded coverage, not a claim that arbitrary software can be
updated safely by guessing a command.

### Apply lane

The apply lane is a working pre-alpha manager executor. It requires an
initialized store, fresh live probe, network-read/write acknowledgement, one
exact action or interactive serial queue, no-rollback acknowledgement where
applicable, and a five-minute phrase.

Initialize the private runtime.zero state once, then update every executable
provider that the host can prove:

```bash
rz0 store init --yes
rz0 updates --apply --all-providers \
  --allow-network-read --allow-network-write --accept-no-rollback
```

For one aggregate candidate, use `--action` with `--all-providers`; this is how
prefix-specific npm actions such as Pi, GSD, and Kilo are selected. Providers
that require system privilege use `/usr/bin/sudo -n`; authenticate first with
`sudo -v`, or start the command from an already elevated shell. runtime.zero
does not collect a password or invoke an interactive helper.

First request a challenge, then repeat the exact command with the emitted phrase
and timestamp. Do not script the phrase or use it across actions.

Execution requires a platform `BoundExecutable` lease before confirmation is
consumed. Linux uses the held `/proc/self/fd` launch identity for direct native
ELF managers; scripts/interpreter chains fail before transaction preparation.
macOS revalidates the direct path's device/inode/link/size/digest immediately
before spawn. Windows uses pre-start Job Object/handle-list containment but
remains pre-alpha pending runtime identity and platform proof. Known self-updaters may replace their own launcher; the executor
accepts that declared transition and relies on fresh provider verification.

A successful supported-platform action records an exact manager write intent,
uses the cancellable bounded process host, revalidates the executable, performs
fresh availability verification, publishes a canonical external-effect receipt,
and only then appends final committed journal evidence. A SIGINT during the
Unix execution lane requests typed cancellation and process-group teardown.
This is not a syscall/filesystem/network sandbox and is not production proof.

The live Mac smoke path has executed OMP and npm-prefix updates successfully and
published committed external-effect receipts. For broad runs, expect a provider
to pause on a failed item and resume from a new fresh review rather than
silently skipping it.

## Recovery status

```bash
rz0 updates --recovery-status \
  --transaction tx.update.<plan-digest>.<timestamp>
```

This command is read-only. It classifies exact immutable journal and external-
effect receipt state as one of:

- no manager write started;
- manager outcome requires fresh verification;
- a verified receipt exists and final journal completion requires explicit
  receipt-bound recovery approval;
- committed receipt/journal agree and no action is indicated;
- evidence conflicts and all automatic action is refused.

It never reruns a manager, edits a receipt, completes a journal, rolls back, or
removes evidence. For the explicit local completion path, use
`rz0 updates --recovery-complete --transaction <id>` to obtain a challenge, then
repeat with its issued timestamp and exact phrase. Follow
[`recovery-guide.md`](recovery-guide.md).

## Local store

```bash
rz0 store plan
rz0 store status
rz0 store init --dry-run
rz0 store init --yes
```

`store init --yes` creates only runtime.zero-owned user-local scaffolding on
currently enabled Unix paths. It refuses unsafe existing state and does not
repair, overwrite, install, activate, or execute modules. Windows creation
remains blocked pending ACL/runtime proof.

## Module staging boundary

The normal module installer is not available. `rz0 modules install --dry-run`
only validates a local package and builds a non-authorizing plan. A separate
developer trial can exercise the foundation's first bounded module-byte write
with a local fixture:

```bash
rz0 modules install --developer-trial --dry-run <package-dir-or-manifest> \
  --signature <envelope.json> --trusted-test-key <key.json> --store-root <path>
```

If the dry-run is valid, repeat the command as `--apply` with its exact
`--challenge-issued-unix-seconds` value and `--confirm` phrase. The trial
accepts only a read-only first-party package, a detached public test-key
signature, and an initialized private store. It verifies source bytes before
copying them, writes only runtime.zero-owned module/receipt state, refuses
replacement, and leaves the installed registry unchanged. It never fetches,
activates, invokes, or executes module code. This is a developer foundation
test, not a public installer or a production trust decision.

## Module surfaces

```bash
rz0 modules
rz0 modules status
rz0 modules status --store-root path/to/store --format json
rz0 modules validate modules/inventory/rz0-module.json
rz0 modules install --dry-run modules/inventory
rz0 modules install --developer-trial --dry-run <package> --signature <envelope.json> \
  --trusted-test-key <key.json> --store-root <path>
rz0 modules lifecycle-plan invoke --dry-run --module-id first-party.inventory \
  --from-state active --to-state active --from-version 0.1.0 --to-version 0.1.0
```

These commands parse, validate, hash, and plan. The developer-trial form is the
exceptional bounded foundation test described above: it can stage verified
read-only fixture bytes after explicit confirmation, but it does not publish a
registry record or make those bytes discoverable, active, or executable. There
is no supported production command to install, activate, invoke, repair,
migrate, upgrade, deactivate, or uninstall module code. The seven first-party
manifests remain planned.

Use `rz0 modules status` when you need the current module-store answer:
valid registry-plus-receipt-plus-installed-byte evidence is
`installed_inactive`; missing or invalid receipt, manifest, or declared package
file evidence is `degraded`; a valid developer-stage receipt and staged byte
set is reported separately as `staged`; the staged receipt must also match its
immutable committed transaction journal and commit receipt, otherwise it is
degraded for review. No module is reported `active` because the lifecycle
execution gate remains unavailable. The output is read-only and path-redacted
by default.

The product direction is broader than this current planning surface: every
feature family or provider should eventually be an independently manageable
module that users can enable or disable for their use case. The target lifecycle
must distinguish installed, enabled, active, degraded/blocked, and action-
authorized states, and disabling must stop module-owned work without deleting
its data. The target `enable`, `disable`, `configure`, `repair`, and
`uninstall` controls are not implemented current commands. See
[`engineering-handoff.md`](engineering-handoff.md) for the end-state catalog,
contract, and next-shift sequence.

## Shell completion

```bash
rz0 completions bash > /tmp/rz0.bash
rz0 completions zsh > /tmp/_rz0
rz0 completions fish > /tmp/rz0.fish
rz0 completions powershell > /tmp/rz0.ps1
```

The command only prints source. Inspect it, then install/source it using your
shell's normal user-local mechanism. `rz0` never edits shell configuration.
Committed copies are under `completions/`; the manual page source is
[`docs/man/rz0.1`](man/rz0.1).

## Next references

- [`troubleshooting.md`](troubleshooting.md)
- [`privacy-and-sharing.md`](privacy-and-sharing.md)
- [`recovery-guide.md`](recovery-guide.md)
- [`platform-notes.md`](platform-notes.md)
- [`documentation-index.md`](documentation-index.md)
- [`completion-checklist.md`](completion-checklist.md)
