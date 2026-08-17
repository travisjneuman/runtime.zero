# Terminal UI Foundation

> The current UX intentionally uses one canonical installed-software list with
> per-item options. Do not recreate separate updater, uninstall, or cleanup lists
> for the same software objects. See
> [`project-status-and-resumption.md`](project-status-and-resumption.md) for the
> current implementation and hardening boundary.

Bare `rz0` opens the terminal UI when both stdin and stdout are interactive and
automation is not detected. `rz0 --tui` explicitly requests that same
full-screen dashboard and returns a clear usage error if the terminal is not
interactive. Explicit subcommands, `--json`, `--format json`, `--no-tui`,
non-interactive pipes/redirects, and automation contexts remain on the
scriptable CLI path.
Scriptable output is written through guarded stdout/stderr handling so common
pipe consumers can stop reading without turning a closed pipe into a user-facing
panic.

The TUI is the primary interactive local-software control surface. It is part
of the foundation, not an optional feature module, and it shares the same
provider discovery, exact action-plan, confirmation, transaction, and
post-update verification contracts as the CLI. The CLI remains the scriptable
and automation-oriented surface.

## Terminal behavior

The interactive TUI uses [`crossterm`](https://crates.io/crates/crossterm) for
raw key handling and terminal restoration, with
[`ratatui`](https://crates.io/crates/ratatui) as the widget/layout renderer for
the full-screen dashboard. `crossterm` remains the terminal-control layer
because it directly solves the foundation requirement that keys such as `q`
must not echo in the terminal. Ratatui was added as a bounded foundation spike
after the custom string renderer proved safe but not product-like enough for
the default bare-`rz0` experience.

The dependency stack is intentionally single-backend: runtime.zero uses
`crossterm 0.29` directly and enables Ratatui's `crossterm_0_29` feature so the
interactive renderer does not pull a second, incompatible terminal stack. The
custom renderer remains in the codebase for scriptable text output and fallback
comparison.

Runtime behavior:

- chooses a named layout tier from terminal dimensions before rendering;
- uses a very-small safe fallback below 50x12;
- uses a compact single-frame dashboard from 50x12 when width/height are constrained;
- uses standard and wide full dashboard layouts from 72x20 and 110x24 respectively;
- keeps the selected section, selected item, and available actions visible in compact mode instead of hiding them behind clipped panes;
- enters raw mode so single-key actions do not require Enter and do not echo;
- uses an alternate screen for the dashboard;
- hides the cursor while active;
- restores raw mode, cursor visibility, and the normal screen on exit or panic
  unwinding through the TUI guard;
- re-renders on terminal resize events;
- treats key press and repeat events as intentional input;
- ignores key release events so Windows terminals do not double-advance navigation;
- clamps layout width/height so narrow terminals do not panic.

Minimum keys:

- `q`: quit safely;
- `r`: refresh the bounded local snapshot;
- `m`: jump to the live system monitor;
- `u`: scan all discovered provider availability sources; this may request
  network metadata but never updates software;
- `U`: update the highlighted installed-software or provider row. runtime.zero
  refreshes exact evidence, shows the manager/target/command, accepts the exact
  challenge phrase in the TUI, and executes the shared update transaction;
- `/`: begin bounded software-name/source/ID search; Enter accepts and Esc cancels;
- `f`: cycle software filters (all, applications, package managers, reviewable);
- `s`: cycle software sort order (name, version, kind);
- Esc: close details/help, back out to navigation, or quit from the base navigation focus;
- `h` or `?`: toggle keyboard/safety help;
- Tab: cycle focus forward through left navigation, details, and command rail;
- Shift+Tab / BackTab: cycle focus backward when exposed by the terminal;
- down/right arrow or `j`: move within the focused region;
- up/left arrow or `k`: move backward within the focused region;
- Enter/Space: open or close details for the selected item or command;
- mouse wheel: scroll the list under the pointer by three rows;
- Home/End: first/last dashboard section in navigation, or first/last row/command in the focused details or command rail.

## Dashboard content

The dashboard performs bounded local reads at startup and shows:

- a live installed-software total;
- direct macOS application bundles and Homebrew formula/cask records;
- deterministic identity groups with explicit provenance/confidence and version
  disagreement, while keeping source records separate;
- versions when bounded bundle/package metadata provides them, plus separate
  service/persistence record counts without placing services in the software list;
- one installed-software list where every row exposes its available details and
  uninstall posture; applicable rows show the exact `rz0 uninstall plan <id>` command;
- cached search/filter/sort controls that do not re-run inventory until `r` is pressed;
- store, registry, receipt, and module lifecycle state;
- the built-in first-party inventory adapter and scriptable `rz0 apps` surface.

The dashboard must not claim planned modules are installed or active. Enter
shows the selected item's details and exact available command. Protected system
software remains blocked. `U` is an explicit, confirmation-bound update action;
it never guesses a provider command and it pauses visibly on unavailable,
blocked, drifted, failed, or recovery-required evidence. `u` remains the
non-mutating provider availability scan.


## Current shell layout

The TUI is intentionally more than a command transcript. The interactive shell
now renders the existing dashboard data model through Ratatui widgets:

- a bounded header panel with product/version and live-inventory status;
- a navigation rail/index for overview, local store, installed software,
  modules, actions, and the system monitor;
- a selected-section panel with the section summary, fixed position counter,
  visible selected row, and a separate details panel;
- foundation state cards for store, registry, receipt, and installed-module
  posture with reusable status-pair formatting;
- a live system monitor section with native CPU, memory, disk, network, uptime,
  and process counters;
- a command rail that lists exact scriptable CLI commands, including `rz0 apps`
  and `rz0 uninstall plan <id>`, with Enter showing the command description;
- an actions footer and optional help overlay; mouse capture is enabled and
  restored on exit.

Interactive rendering applies Dossier Navy / Burnished Brass status tones to
headers, selected navigation, status badges, and action rows. Reusable Ratatui
component helpers own the header, state cards, details panel, command rail, and
actions footer so later visual tuning stays narrow. Text labels remain the source of truth: `[OK]`, `[INFO]`, `[PLAN]`,
`[DRY-RUN]`, `[BLOCKED]`, and `[SKIP]` must still explain the state when color
is disabled or unavailable.

Color control is global:

- `--color=auto` is the default and respects `NO_COLOR`;
- `--color=never` disables ANSI, including in the interactive TUI;
- `--color=always` forces color for supported human-readable surfaces;
- JSON output must stay ANSI-free regardless of color mode.

The text dashboard shown by `rz0 --no-tui` uses the same data model but keeps
the custom text renderer without raw-mode terminal control. That keeps the CLI
path scriptable while letting the interactive TUI use a stronger widget/layout
layer.

## Dashboard JSON contract

`rz0 --json` exposes the same foundation dashboard state as a machine-readable
contract. The contract is additive and currently schema version `1`.

Required top-level fields for `rz0 --json`:

- `schema_version: 1`;
- `contract: "foundation_dashboard"`;
- `read_only: true`;
- `writes_attempted: false`;
- product identity fields such as `title`, `command`, `version`, and `mode`;
- store, registry, receipt, store-init, installed-module, and planned-module
  summary fields used by the TUI and text dashboard;
- section rows whose visible labels remain the meaning source.

JSON output must never include ANSI escape sequences and must not depend on
terminal dimensions, color mode, raw mode, or Ratatui rendering state. The
scriptable dashboard remains a read-only snapshot; an interactive TUI update
changes the live TUI state to `writes_attempted: true` only after the shared
executor publishes a verified receipt.

## Known TUI limitations

- Uninstall, cleanup, integrity remediation, and module lifecycle are reviews or
  unavailable, not interactive actions; privacy-reviewed report output is CLI-only.
- Search/filter/sort operate on the cached inventory until `r` refreshes it.
- The monitor's metric depth varies by platform and first-sample CPU values may
  show `sampling`.
- Automated buffer/PTY tests do not replace real terminal, keyboard, mouse,
  screen-reader, SSH, tmux/screen, Windows Console/Terminal, and human review.
- A manual page, shell completions, and operator recovery guide now exist.
  Direct TUI rollback/recovery completion, localization, and human
  accessibility review remain incomplete; cancellation is surfaced while an
  update worker is running and recovery status remains available from the CLI.

## Website parity backlog

The terminal TUI is now the source of truth for labels, state hierarchy,
responsive layout vocabulary, and interactive action entry points. The current
public-site mock predates the six-section TUI, update review, and native monitor.
Website source should be updated only in a separate approved deployment lane.
See [`website-tui-parity-backlog.md`](website-tui-parity-backlog.md) for the exact
backlog and checks.

## Final-artifact PTY smoke

`scripts/smoke_terminal_artifact.py` exercises an already built single-link
binary rather than a source checkout. It creates bounded pseudo-terminals,
selects explicit TERM values and dimensions, injects a resize in the compact
case, sends one raw `q`, and requires clean exit plus balanced alternate-screen
entry/exit without captured line-echo. It stores only bounded output digests and
counts, never terminal contents or host paths, and cannot authorize release.

The current universal macOS artifact passed four `xterm-256color`, `xterm`,
`screen`, and `vt100` PTY cases through both ARM64 and Rosetta x86-64 slices,
including 40×12→100×30 resize. This is final-artifact local smoke, not proof for
Terminal.app/iTerm/etc. versions, Intel hardware, older macOS, Windows console
stacks, Linux terminal families, screen readers, or human accessibility.

## Verification expectations

Automated tests should cover launch routing, key-event filtering, reducer
state, no ANSI in plain text output, selected-section rendering, narrow terminal
rendering, help output, and visible-width invariants across compact, normal,
wide, colorized, and non-colorized frames. Renderer tests should also exercise
every dashboard section across help and non-help states so future visual polish
does not accidentally hide the text labels that make color optional. Ratatui
buffer tests should prove labels remain visible with and without color and that
compact/normal/wide frames stay within terminal boundaries. Dashboard JSON
tests should prove the versioned read-only contract fields remain present and
ANSI-free. A manual smoke check is still required after local install refresh
because full-screen raw terminal behavior depends on the host terminal
emulator.

Manual check after refreshing the installed binary:

1. Run `rz0` in a new interactive PowerShell terminal.
2. Press down arrow once while the left navigation is focused; selection should advance exactly one section.
3. Hold down arrow; repeat navigation should continue predictably.
4. Press Tab and Shift+Tab; focus should move visibly among left navigation, details, and command rail.
5. In details or command focus, press Enter/Space; the details panel should appear.
6. Scroll the mouse over the installed-software list; the selected row should
   advance three rows per wheel event and remain visible at the bottom.
7. Press `m`; the system monitor section should show live native resource and
   process counters and refresh once per second.
8. Press `u`; all discovered provider lanes should render update candidates or
   explicit unavailable/delegated/observed-only warnings.
9. Highlight a planned candidate and press `U`; the TUI should show the exact
   manager, target, command, and challenge phrase. Type the phrase and press
   Enter to execute, or Esc to cancel.
10. Press `r`; the local snapshot should refresh.
11. Press Esc; details/help/confirmation should close or focus should back out
   before quitting.
12. Press `h` or `?`; help should toggle without typed input echo.
13. Press `q`; the TUI should exit and restore the normal prompt.

## Brand and maintainability

TUI visual tokens are centralized in `src/tui_theme.rs` and use the
`BRAND.md` Dossier Navy / Burnished Brass direction. Labels and colors are
secondary to clear text so the dashboard remains usable over SSH, in restricted
terminals, and with color disabled through `NO_COLOR`.

Rendering, app state, input handling, and data shaping are deliberately split:

- `src/tui_dashboard.rs` builds the bounded dashboard data model;
- `src/tui_canvas.rs` owns frame, padding, truncation, and line helpers;
- `src/tui_render.rs` renders the resize-safe scriptable text dashboard shell;
- `src/tui_render_support.rs` owns render-only text helpers and tone mapping;
- `src/tui_ratatui.rs` composes the interactive widget dashboard;
- `src/tui_layout.rs` owns named layout tiers and minimum terminal dimensions;
- `src/tui_ratatui_components.rs` owns reusable header, state card, details,
  compact, and actions-footer components;
- `src/tui_ratatui_rail.rs` renders the command rail;
- `src/tui_ratatui_support.rs` owns Ratatui style/layout helper primitives;
- `src/tui_command_rail.rs` owns command preview metadata;
- `src/tui_state.rs` owns focus, navigation, details, mouse-scroll,
  search/filter/sort, and help state transitions;
- `src/tui_app.rs` owns terminal raw-mode lifecycle and event handling;
- `src/tui_theme.rs` owns tokens/status label constants.

Future TUI polish should preserve this separation so the website reference and
real terminal UI can evolve together without making terminal usability depend
on a web layout.

Website TUI parity remains a later website-lane slice after the real terminal TUI stabilizes. The terminal TUI should continue to be the source of truth when terminal usability and the website mock differ; do not edit site/ as part of foundation TUI polish unless that lane is explicitly approved.
