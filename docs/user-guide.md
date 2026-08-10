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
blocks. On macOS, manager apply currently fails before transaction preparation
because no reviewed exact opened-artifact-to-spawn primitive exists. Windows
also remains blocked by production process containment.

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

The six sections are overview, local store, installed software, modules,
actions, and system monitor. Important controls:

- `r`: refresh local inventory;
- `u`: explicitly query manager availability and potentially read network
  metadata; it never applies an update;
- `m`: jump to the one-second monitor view;
- `/`: search; `f`: filter; `s`: sort;
- arrows or `j`/`k`: move; Home/End: boundaries;
- Tab/Shift+Tab: change focus; Enter/Space: details;
- `h`/`?`: help; Esc: back; `q`: quit.

The TUI has no independent mutation authority. Action rows hand off to exact CLI
commands so confirmation, transaction, and recovery policy is not duplicated.
See [`tui.md`](tui.md).

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
```

The executable is opened directly, bounded, hashed, identified, and sealed into
the resulting plan. A second invocation must reproduce the same plan identity;
manager executable replacement invalidates confirmation.

### Apply lane

The apply lane remains pre-alpha and unsupported. It requires an initialized
store, fresh live probe, network-read/write acknowledgement, one exact action or
interactive serial queue, no-rollback acknowledgement where applicable, and a
five-minute phrase.

First request a challenge, then repeat the exact command with the emitted phrase
and timestamp. Do not script the phrase or use it across actions.

Execution now requires a platform `BoundExecutable` lease before confirmation is
consumed. Linux uses the held `/proc/self/fd` launch identity for direct native ELF
managers; scripts/interpreter chains fail before transaction preparation. macOS
fails closed
before transaction creation because an exact mechanism is not implemented.
Windows remains fail-closed at production handle/process containment.

A successful supported-platform action records an exact manager write intent,
uses the cancellable bounded process host, revalidates the executable, performs
fresh availability verification, publishes a canonical external-effect receipt,
and only then appends final committed journal evidence. A SIGINT during the
Unix execution lane requests typed cancellation and process-group teardown.
This is not a syscall/filesystem/network sandbox and is not production proof.

Never test a real update on a normal workstation. Mutation evidence belongs on
snapshot-backed disposable hosts with synthetic, noncritical packages.

## Recovery status

```bash
rz0 updates --recovery-status \
  --transaction tx.update.<plan-digest>.<timestamp>
```

This command is read-only. It classifies exact immutable journal and external-
effect receipt state as one of:

- no manager write started;
- manager outcome requires fresh verification;
- a verified receipt exists but final journal completion needs a future exact
  recovery approval path;
- committed receipt/journal agree and no action is indicated;
- evidence conflicts and all automatic action is refused.

It never reruns a manager, edits a receipt, completes a journal, rolls back, or
removes evidence. Follow [`recovery-guide.md`](recovery-guide.md).

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

## Module surfaces

```bash
rz0 modules
rz0 modules validate modules/inventory/rz0-module.json
rz0 modules install --dry-run modules/inventory
```

These commands parse, validate, hash, and plan. They do not install, activate,
invoke, repair, migrate, upgrade, deactivate, or uninstall module code. The
seven first-party manifests remain planned.

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
