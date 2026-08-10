# Troubleshooting

This is a pre-alpha operational guide. Prefer read-only inspection, preserve
partial evidence, and stop before guessing at a repair.

## General triage

```bash
rz0 --version
rz0 doctor --format json
rz0 report --format json
```

If a command fails, record its exit code. Do not pipe private output into a public
paste service. Add `RUST_BACKTRACE=1` only for a locally reproduced development
panic; review the result before sharing it.

## Bare `rz0` did not open the TUI

This is expected when stdin/stdout are not terminals, `CI`/`NO_COLOR`/`TERM=dumb`
is set, or `RZ0_NO_TUI` is present. Use:

```bash
rz0 --no-tui
rz0 --json
rz0 doctor
```

For interactive use, run directly in a terminal that supports alternate-screen
and keyboard input. The TUI refuses unsafe/noninteractive startup rather than
emitting escape sequences into automation.

## Terminal display is corrupted

Press `q`, then run `reset` using your shell/terminal if needed. Avoid launching
inside an output-capturing pipe. Record terminal name/version for a bug report,
but do not include environment values wholesale.

## Inventory source is `partial` or `unavailable`

This is not silently converted to an empty successful source. Common causes are:

- expected metadata roots do not exist on that host;
- permissions deny an optional source;
- a source record was malformed or oversized;
- bounded record/warning ceilings were reached;
- the platform collector is intentionally shallow.

Review `sources[]` and source-local warnings. Do not elevate the whole command.
Do not use `sudo rz0 scan` as a workaround. Missing optional sources can be a
normal host condition.

## Inventory or TUI refresh is slow

The current collector walks bounded application/package/service metadata.
Service-heavy macOS hosts can contain hundreds of launchd records. Capture the
source `duration_ms` fields locally. Do not claim a performance regression from
a debug build; reproduce with a release artifact and the committed benchmark
harness.

The TUI refreshes inventory only on startup or explicit `r`; monitor samples use
a separate one-second cadence. The app catalog does not invoke package managers.

## Software appears duplicated or grouped incorrectly

Run `rz0 apps --format json` and inspect `identifiers` and `source_ids`.

- A shared source identifier groups records deterministically.
- Name-normalized groups are only heuristic.
- Conflicting versions are preserved and shown as disagreement.
- Similar names with different source identities should remain separate.

Do not edit installed software or manager metadata to force a visual merge.
Report the smallest redacted synthetic reproduction.

## `rz0 report` is blocked

The report fails closed if inventory is unredacted, a source was not represented,
the summary exceeds ceilings, a privacy-forbidden warning/path/name leaks into
the result, or diagnostics contain disallowed fields. Re-run without
`--include-raw-paths` (the report command never enables it itself) and inspect
`rz0 doctor` plus redacted `rz0 scan --dry-run` locally.

A blocked report should not be bypassed by copying raw scan output into an issue.

## Update probe cannot find a manager

Use an exact absolute executable path in an allowlisted location. Runtime.zero
does not trust PATH lookup for live manager execution. Typical development paths
include manager-owned standard prefixes such as `/opt/homebrew/bin/brew` or
`/usr/bin/apt` when that adapter is supported.

A symlink, non-regular file, multi-link executable, oversized artifact,
untrusted parent chain, or replaced file is refused. Do not copy a manager binary
to a temporary path to bypass this policy.

## Update probe reports network acknowledgement required

Live availability is network-capable and must be explicit:

```bash
rz0 updates --dry-run --probe --manager <id> \
  --executable <absolute-path> --allow-network-read
```

This permits only the selected bounded manager query. It is not a general network
sandbox or a promise that the manager will contact only one endpoint.

## Confirmation phrase is rejected

Phrases are exact, action-scoped, plan-bound, executable-bound, short-lived, and
single-use. Regenerate the challenge after any plan/executable change or expiry.
Do not trim, script, save, or reuse a phrase. Queue actions each receive an
independent phrase.

## Apply is blocked on macOS

Expected current behavior. macOS observation can seal executable identity, but
no reviewed exact opened-artifact-to-spawn mechanism is implemented. Apply fails
before transaction preparation. Do not substitute a pathname spawn or disable
the check.

## Apply is blocked on Windows

Expected current behavior. Race-free process-tree containment and executable
handle-to-process binding are not production-complete. Discovery/review can still
run; mutation must stay disabled.

## Apply was interrupted or outcome is uncertain

Do not rerun the manager. Use the exact transaction ID:

```bash
rz0 updates --recovery-status --transaction <id>
```

Then follow [`recovery-guide.md`](recovery-guide.md). Preserve receipt/journal
state and inspect manager-native read-only installed/availability state.

## Store initialization refuses existing state

Run:

```bash
rz0 store plan
rz0 store status
rz0 store init --dry-run
```

The initializer refuses symlinks, wrong-type entries, unsafe ownership/mode, and
invalid registry/marker content. It will not repair or overwrite them. Do not
remove existing paths unless ownership and purpose are independently proven.
Windows store creation is intentionally unsupported.

## Module command does not install or run anything

Expected. Module commands currently validate and plan only. The manifests and
lifecycle contracts do not grant execution authority.

## Completion source does not match the CLI

Regenerate from the same binary:

```bash
rz0 completions bash
rz0 completions zsh
rz0 completions fish
rz0 completions powershell
```

Committed completion files are generated artifacts. `rz0 --help` remains parser
authority. If drift exists, report both binary commit and file commit.

## Build or validation failure

Run the smallest checks first:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

Then use the target commands in [`CONTRIBUTING.md`](../CONTRIBUTING.md). Missing
`cargo-audit`, `cargo-deny`, cross-target toolchains, native runners, or system
libraries must be reported as unavailable evidence, not silently treated as a
pass.

## Reporting a bug safely

Include the smallest reviewed facts: source commit/version, platform family,
command shape, exit code, expected vs actual behavior, whether state may have
changed, and a synthetic reproduction when possible. Start with
`rz0 report --format json`; never attach raw transaction state by default.
