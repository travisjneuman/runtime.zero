# Terminal UI guide

`rz0` opens the interactive terminal UI only when stdin and stdout are real
terminals and automation is not detected. Use `rz0 --tui` to request it
explicitly, `rz0 --no-tui` for deterministic text, and `rz0 --json` for the
read-only machine-readable dashboard. Explicit subcommands never launch the
TUI.

The TUI is the Rust-first Dossier Queue presentation of the same evidence,
provider, action-plan, confirmation, transaction, cancellation, receipt, and
verification contracts used by the CLI. It is not a second authority path.
The repository remains pre-alpha; this guide records source and test evidence,
not owner, platform, or public-release acceptance.

## First frame and loading

The program renders a small local-control shell before collecting the full
software inventory and system snapshot. The initial frame says `loading local
snapshot`; the background read then replaces it with the completed local
dashboard. A disconnected worker fails closed and is shown as unavailable. No
automatic retry occurs: `r` is an explicit refresh request. `q` cancels and
drops an in-flight startup or refresh worker before leaving the TUI. Pressing
`r` cancels the previous load, increments a generation, and starts one new
worker; a late result from an older generation cannot replace the newer
snapshot. The refresh frame explicitly says `refreshing local snapshot` so the
request is visible even when the previous load has not completed yet.

The shell is local and read-only until an exact provider action is reviewed,
confirmed, executed, and freshly verified. Loading, ready, unavailable, empty,
blocked, stale, and failed are separate states; an empty result is not treated
as “not loaded.” Provider review is explicit (`u`) and cancellable, so a slow
provider cannot block the ready local first screen.
Provider probes run through the Rust process host in a detached Unix session so
terminal-aware tools such as AIUP cannot reopen `/dev/tty` and overwrite the
runtime.zero frame.

## Workspaces

The UI has five stable destinations:

- `OVERVIEW`: local readiness, attention, and the next safe step;
- `EXPLORE`: searchable local evidence and provider observations;
- `REVIEW`: exact action plans, findings, blocked boundaries, and confirmation
  requirements;
- `ACTIVITY`: running, cancellation, receipt, stale, and recovery evidence;
- `MODULES`: registry-backed module posture without lifecycle authority.

The Diagnostics workspace derives module lifecycle counts from the same
registry/receipt status contract as `rz0 modules status`: valid evidence is
shown as installed-inactive, receipt or registry problems are shown as
degraded, valid developer-stage evidence is shown as staged, and the workspace
  explicitly says lifecycle execution is unavailable. Staged evidence that fails
the receipt, immutable transaction-journal, commit-receipt, or byte checks is
counted separately as requiring review rather than being shown as valid staged
material.
It never renders an active-module claim or adds activation/invocation controls.
The selected Diagnostics evidence also exposes the effective built-in policy
digest and the compact statement that network, production modules, shell
execution, telemetry, and automatic lifecycle work remain disabled. The same
digest is available from `rz0 config --format json`.

The same bounded Dossier Queue shell is reused in every destination. Modules
contribute typed records and references only; they cannot add global chrome,
keymaps, focus regions, lifecycle state, confirmation, or execution authority.

The primary list is intentionally concise: Home shows the next provider-review
decision and small toolchain/software counts, while selected details carry
dense evidence such as identity groups, load averages, journal counts,
active-use uncertainty, and integrity posture. This keeps the interface calm
without hiding information from the keyboard-accessible selected pane or the
scriptable CLI/JSON surfaces.

Home and Toolchain also expose the Rust-owned AIUP boundary from the same local
catalog: orchestrator posture, AI-tool count, and provider-review count remain
visible as a compact summary, while `rz0 aiup --format json` provides the full
scriptable review. Toolchain rows also include the same bounded named
executable evidence as `rz0 scan`; unknown or wrapper-like PATH names remain
observations and do not become provider actions. This is discovery and posture
only; it does not invoke AIUP or create a second provider action path.

## Layout and terminal behavior

The TUI uses `crossterm` for raw terminal control and Ratatui for the
interactive layout. The scriptable text renderer uses the same dashboard model
without raw mode. There are never more than two bordered content panels:

```text
runtime.zero / OVERVIEW                         ready
attention first · choose the next safe step
[1 Overview] [2 Explore] [3 Review] [4 Activity] [5 Modules]
┌ Overview · evidence queue ─────────────┐ ┌ Selected detail ─────────┐
│ > [OK] local snapshot ready             │ │ source: overview         │
│   [PLAN] provider review not requested   │ │ state: read-only         │
└─────────────────────────────────────────┘ └───────────────────────────┘
status · job idle
↑↓/jk move · Tab focus · Enter detail/review · c confirm · q quit
```

Named layout tiers keep content bounded:

- below `50x12`: a safe notice and the `rz0 --no-tui` escape hatch;
- compact: the primary list and selected context stack without clipped
  controls;
- standard and wide: the same two surfaces with more room for explanations.

The terminal guard enters raw mode and the alternate screen, hides the cursor,
handles resize and key-repeat events, ignores key-release navigation, and
restores raw mode, cursor visibility, mouse capture, and the normal screen on
exit or panic unwinding. `q` cancels an active provider review or fresh action
preparation before leaving. `NO_COLOR` and `--color=never` preserve all semantic
labels without ANSI; JSON is always ANSI-free.

## Controls

- `q`: quit safely;
- `r`: explicitly refresh the local snapshot;
- `u`: perform a read-only provider-availability review;
- `c`: request the foundation-owned confirmation challenge for the selected
  reviewable action;
- `/`: search cached software records; Enter accepts and Esc cancels;
- `Tab` / `Shift+Tab`: cycle navigation, selected details, and context pane;
- arrows or `j`/`k`: move within the focused region;
- `Home` / `End`: jump to the relevant boundary;
- `Enter`: open detail or the read-only action review;
- mouse click: select a route/record or open detail; wheel moves the bounded list;
- `h` / `?`: open help; `Esc`: close details/help/search/confirmation before
  backing out or quitting.

Review is an entry point, not permission to write. The TUI shows the exact
foundation action reference, plan and write-set digests, target, risk,
capabilities, network/elevation requirements, rollback/recovery posture, and
action ID before it requests the short-lived exact confirmation phrase.
Cancellation and failed or stale evidence remain visible and read-only.

## Evidence and provider posture

Explore and Review consume evidence-backed provider records rather than
assuming that a display name owns an update channel. Each action record is
derived from the foundation `ActionPlan` and carries sealed plan/write-set
digests, risk, capabilities, confirmation, identity, rollback, and recovery
posture. A planned update is re-prepared against fresh evidence by the
existing foundation function before any challenge or execution; the UI never
constructs or validates a plan itself.

The `u` review may request bounded network metadata according to the existing
CLI policy, but it never writes. A planned update still requires the shared
exact executable binding, plan-bound confirmation, transaction evidence, fresh
post-action verification, and receipt/recovery state. The TUI cannot bypass
those gates.

## Read-only JSON parity

`rz0 --json` remains a versioned `foundation_dashboard` snapshot. It includes
the same five sections, rows, visible labels, store/registry/receipt/module
posture, provider-review status, and update counters used by the TUI. Terminal
dimensions, color, raw mode, and Ratatui state must not affect the JSON shape.
The dashboard JSON also exposes `inactive_module_count`,
`degraded_module_count`, and `module_lifecycle_execution_available`; these are
status fields only and do not authorize lifecycle actions. It also exposes
`configuration_sha256`, which identifies the immutable built-in policy in force
without exposing host paths or loading user configuration.

## Validation contract

Automated coverage includes reducer focus/navigation behavior, loading,
unavailable, empty, and blocked failed-closed states, plain/color text parity,
JSON ANSI exclusion, Ratatui buffer bounds, all five workspaces,
help/search/confirmation overlays, PTY restoration, and the shared update
plan/action IDs. The required local checks are:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo run --locked -- doctor
cargo run --locked -- scan --dry-run
```

Automated buffer tests do not replace review in real terminals, SSH/tmux,
Windows Terminal/Console, macOS terminals, Linux terminals, screen readers,
and at the supported compact sizes. The release gate also requires a final
artifact PTY smoke and explicit owner acceptance; neither is implied by a
passing Rust test.

## Implementation boundaries

- `src/tui_dashboard.rs` remains a foundation-owned bounded snapshot builder
  used by CLI/text and the typed adapter; it is not an interactive renderer;
- `src/tui_render.rs` is the retained scriptable text contract for `--no-tui`;
- `src/ui/model.rs` and `src/ui/foundation_adapter.rs` define bounded typed
  records and projections from foundation evidence/action plans;
- `src/ui/messages.rs` and `src/ui/state.rs` define the reducer, focus,
  overlays, search, confirmation input, stale generations, and job states;
- `src/ui/layout.rs`, `src/ui/widgets.rs`, `src/ui/screens/`, and
  `src/ui/theme.rs` own pure Ratatui composition;
- `src/ui/terminal.rs` owns raw-mode lifecycle and one-at-a-time cancellable
  workers that delegate review/prepare/execute to foundation contracts;
- `src/ui/testkit.rs` owns deterministic TestBackend fixtures and buffer
  assertions.

The old command-rail component/module is retired. Future UI work should extend
typed workspace/provider/action states and shared CLI contracts rather than
reintroducing parallel command catalogs or permanent chrome.
