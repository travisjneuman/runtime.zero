# Terminal UI guide

`rz0` opens the interactive terminal UI only when stdin and stdout are real
terminals and automation is not detected. Use `rz0 --tui` to request it
explicitly, `rz0 --no-tui` for deterministic text, and `rz0 --json` for the
read-only machine-readable dashboard. Explicit subcommands never launch the
TUI.

The TUI is a Rust-first presentation of the same evidence, provider, action
plan, confirmation, transaction, cancellation, receipt, and verification
contracts used by the CLI. It is not a second authority path. The current
repository is still pre-alpha; this guide describes the active implementation
contract and does not claim public-release acceptance by itself.

## First frame and loading

The program renders a small local-control shell before collecting the full
software inventory and system snapshot. The initial frame says `loading local
snapshot`; the background read then replaces it with the completed local
dashboard. A disconnected worker fails closed and is shown as unavailable. No
automatic retry occurs: `r` is an explicit refresh request.

The shell is local and read-only until an exact provider action is reviewed,
confirmed, executed, and freshly verified. Loading, unavailable, empty, and
blocked are separate states; an empty result is not treated as “not loaded.”
Provider probes run through the Rust process host in a detached Unix session so
terminal-aware tools such as AIUP cannot reopen `/dev/tty` and overwrite the
runtime.zero frame.

## Workspaces

The dashboard has five stable destinations:

- `HOME`: local readiness, update-review state, and the next safe step;
- `TOOLCHAIN`: Rust, AI, developer-tool, and provider-owned records, including
  AIUP-managed candidates when the provider evidence identifies them;
- `SOFTWARE`: installed applications and packages outside the toolchain;
- `SYSTEM`: bounded CPU, memory, disk, network, and process evidence;
- `DIAGNOSTICS`: store, registry, receipt, module, and recovery posture.

The same two-panel shell is reused in every workspace. Home and the other
workspaces do not expose a persistent command rail, duplicate status-card
dashboard, or standalone Actions destination.

## Layout and terminal behavior

The TUI uses `crossterm` for raw terminal control and Ratatui for the
interactive layout. The scriptable text renderer uses the same dashboard model
without raw mode. There are never more than two bordered content panels:

```text
runtime.zero / LOCAL CONTROL                         [ready]
local snapshot · 273 software · no action runs without confirmation

HOME   TOOLCHAIN   SOFTWARE   SYSTEM   DIAGNOSTICS

┌ HOME / NEXT STEP ───────────────────────┐ ┌ SELECTED ────────────────┐
│ one focused list or task summary         │ │ source, state, next step │
└──────────────────────────────────────────┘ └──────────────────────────┘

status                                                        [? help]
↑↓/jk move · Tab focus · Enter details · Review action [U] · q quit
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
- `m`: select `SYSTEM`;
- `u`: perform a read-only provider-availability review;
- `U`: compatibility shortcut for `Review action`; it acts only on the exact
  selected planned update through the shared safety path;
- `/`: search cached software records; Enter accepts and Esc cancels;
- `f`: cycle software filters; `s`: cycle software sort order;
- `Tab` / `Shift+Tab`: cycle navigation, selected details, and context pane;
- arrows or `j`/`k`: move within the focused region;
- `Home` / `End`: jump to the relevant boundary;
- `Enter` / `Space`: open or close the selected explanation;
- mouse wheel: move the list under the pointer by a bounded increment;
- `h` / `?`: open help; `Esc`: close details/help/search/confirmation before
  backing out or quitting.

`Review action [U]` is a review entry point, not permission to write. The TUI
must show the provider, exact target, executable identity, command, network and
elevation requirements, rollback/recovery posture, action ID, and plan digest
before it requests the short-lived exact confirmation phrase. Cancellation and
failed or stale evidence remain visible and read-only.

## Evidence and provider posture

The Toolchain workspace groups records by evidence-backed provider rather than
assuming that a display name owns an update channel. Each tool row now includes
the bounded provider ID in its value and selected explanation, including
`aiup` for AIUP-managed evidence. It may show native AIUP,
Cargo, rustup, npm-prefix, Homebrew, or other discovered provider records. Each
record must remain visibly distinguishable as ready, update available,
delegated, unavailable, observed-only, blocked, or unsupported. A provider that
is not installed or not currently supported is reported as such; it is not
silently promoted into an executable action.

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

## Validation contract

Automated coverage must include reducer focus/navigation behavior, loading and
failed-closed states, plain/color text parity, JSON ANSI exclusion, Ratatui
buffer bounds, all five workspaces, help/search/confirmation overlays, PTY
restoration, and the shared update plan/action IDs. The required local checks
are:

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

- `src/tui_dashboard.rs` builds the bounded, serializable dashboard model and
  provider/toolchain grouping;
- `src/tui_render.rs` renders the scriptable text shell;
- `src/tui_ratatui.rs` renders the interactive shell;
- `src/tui_layout.rs`, `src/tui_canvas.rs`, and the `*_support.rs` modules own
  layout, truncation, and style primitives;
- `src/tui_state.rs` owns focus, navigation, details, search, confirmation,
  and read-only state transitions;
- `src/tui_app.rs` owns raw-mode lifecycle, event dispatch, background startup
  loading, provider review workers, cancellation, and terminal restoration.

The old command-rail component/module is retired. Future UI work should extend
typed workspace/provider/action states and shared CLI contracts rather than
reintroducing parallel command catalogs or permanent chrome.
