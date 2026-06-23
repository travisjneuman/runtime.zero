# Terminal UI Foundation

Bare `rz0` opens the terminal UI when both stdin and stdout are interactive and
automation is not detected. `rz0 --tui` explicitly requests that same
full-screen dashboard and returns a clear usage error if the terminal is not
interactive. Explicit subcommands, `--json`, `--format json`, `--no-tui`,
non-interactive pipes/redirects, and automation contexts remain on the
scriptable CLI path.
Scriptable output is written through guarded stdout/stderr handling so common
pipe consumers can stop reading without turning a closed pipe into a user-facing
panic.

The TUI is a safe review dashboard. It is part of the foundation, not an
optional feature module, but it does not replace the CLI contracts. Every
capability shown in the TUI must remain available through stable text or JSON
commands.

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
- keeps focus, section, read-only, and preview-only labels visible in compact mode instead of hiding actions behind clipped panes;
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
- Esc: close preview/help, back out to navigation, or quit from the base navigation focus;
- `h` or `?`: toggle keyboard/safety help;
- Tab: cycle focus forward through left navigation, details, and command rail;
- Shift+Tab / BackTab: cycle focus backward when exposed by the terminal;
- down/right arrow or `j`: move within the focused region;
- up/left arrow or `k`: move backward within the focused region;
- Enter/Space: toggle a read-only details or command preview, never execution;
- Home/End: first/last dashboard section when left navigation is focused.

## Dashboard content

The dashboard is read-only and may show only foundation state:

- foundation safety posture;
- store initialization readiness or initialized state;
- store path/status summary;
- installed-module registry validity;
- receipt validation state;
- installed module count;
- planned first-party module family count;
- dry-run-only module install posture.

The dashboard must not claim planned modules are installed or active, must not
run module code, and must not trigger installs, updates, cleanup, remote
fetches, or destructive actions.


## Current shell layout

The TUI is intentionally more than a command transcript. The interactive shell
now renders the existing dashboard data model through Ratatui widgets:

- a bounded header panel with product/version, brand posture, and read-only status badge;
- a navigation rail/index for numbered dossier sections: foundation, local
  store, modules, and safety gates;
- a selected-section panel with dossier code, summary, current position,
  visible details focus, and read-only row previews;
- foundation state cards for store, registry, receipt, and installed-module
  posture with reusable status-pair formatting;
- a command rail that supports selection and read-only previews of equivalent
  scriptable CLI commands without running them, with explicit `PREVIEW ONLY`
  copy;
- a persistent safety footer and optional help overlay.

Interactive rendering applies Dossier Navy / Burnished Brass status tones to
headers, selected navigation, focus titles, status badges, and blocked/dry-run
rows. Reusable Ratatui component helpers own the header, state cards,
preview-only copy, command rail, and safety footer so later visual tuning stays
narrow. Text labels remain the source of truth: `[OK]`, `[INFO]`, `[PLAN]`,
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

Required top-level fields:

- `schema_version: 1`;
- `contract: "foundation_dashboard"`;
- `read_only: true`;
- `writes_attempted: false`;
- product identity fields such as `title`, `command`, `version`, and `mode`;
- store, registry, receipt, store-init, installed-module, and planned-module
  summary fields used by the TUI and text dashboard;
- section rows whose visible labels remain the meaning source.

JSON output must never include ANSI escape sequences and must not depend on
terminal dimensions, color mode, raw mode, or Ratatui rendering state.

## Website parity backlog

The terminal TUI is now the source of truth for labels, state hierarchy,
responsive layout vocabulary, and read-only command preview posture. Website
mockups should be updated only in a separate website lane after Travis approves
the visual direction. See [`website-tui-parity-backlog.md`](website-tui-parity-backlog.md)
for the exact backlog and checks.
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
5. In details or command rail focus, press Enter/Space; a read-only preview should appear and no command should run.
6. Press Esc; preview/help should close or focus should back out before quitting from base navigation.
7. Press `h` or `?`; help should toggle without typed input echo and show focus-region guidance.
8. Press `q`; the TUI should exit and restore the normal prompt.

## Brand and maintainability

TUI visual tokens are centralized in `src/tui_theme.rs` and use the
`BRAND.md` Dossier Navy / Burnished Brass direction. Labels and colors are
secondary to clear text so the dashboard remains usable over SSH, in restricted
terminals, and with color disabled through `NO_COLOR`.

Rendering, app state, input handling, and data shaping are deliberately split:

- `src/tui_dashboard.rs` builds the read-only data model;
- `src/tui_canvas.rs` owns frame, padding, truncation, and line helpers;
- `src/tui_render.rs` renders the resize-safe scriptable text dashboard shell;
- `src/tui_render_support.rs` owns render-only text helpers and tone mapping;
- `src/tui_ratatui.rs` composes the interactive widget dashboard;
- `src/tui_layout.rs` owns named layout tiers and minimum terminal dimensions;
- `src/tui_ratatui_components.rs` owns reusable header, state card,
  preview-only, compact, and safety-footer components;
- `src/tui_ratatui_rail.rs` renders the read-only command preview rail;
- `src/tui_ratatui_support.rs` owns Ratatui style/layout helper primitives;
- `src/tui_command_rail.rs` owns command preview metadata;
- `src/tui_state.rs` owns focus, navigation, preview, and help state transitions;
- `src/tui_app.rs` owns terminal raw-mode lifecycle and event handling;
- `src/tui_theme.rs` owns tokens/status label constants.

Future TUI polish should preserve this separation so the website reference and
real terminal UI can evolve together without making terminal usability depend
on a web layout.

Website TUI parity remains a later website-lane slice after the real terminal TUI stabilizes. The terminal TUI should continue to be the source of truth when terminal usability and the website mock differ; do not edit site/ as part of foundation TUI polish unless that lane is explicitly approved.
