# Terminal UI Foundation

Bare `rz0` opens the terminal UI when both stdin and stdout are interactive and
automation is not detected. Explicit subcommands, `--json`, `--format json`,
`--no-tui`, non-interactive pipes/redirects, and automation contexts remain on
the scriptable CLI path.

The TUI is a safe review dashboard. It is part of the foundation, not an
optional feature module, but it does not replace the CLI contracts. Every
capability shown in the TUI must remain available through stable text or JSON
commands.

## Terminal behavior

The current TUI uses [`crossterm`](https://crates.io/crates/crossterm) for raw
key handling and terminal restoration. `crossterm` was chosen because it is a
mainstream Rust terminal crate, keeps the dependency surface smaller than a full
widget framework, and directly solves the foundation requirement that keys such
as `q` must not echo in the terminal.

Runtime behavior:

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

- `q` or `Esc`: quit safely;
- `h` or `?`: toggle keyboard/safety help;
- `Tab`, down arrow, right arrow, or `j`: next dashboard section;
- up arrow, left arrow, BackTab, or `k`: previous dashboard section;
- Home: first dashboard section;
- End: last dashboard section.

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

The TUI is intentionally more than a command transcript. The current shell
renders:

- a header with product/version and foundation mode;
- a navigation rail for numbered dossier sections: foundation, local store,
  modules, and safety gates;
- a selected-section panel with dossier code, summary, current position, and
  focused rows;
- foundation state cards for store, registry, receipt, and installed-module
  posture;
- a command rail that points back to scriptable CLI commands;
- a persistent safety footer and optional help text.

The text dashboard shown by `rz0 --no-tui` uses the same data/rendering model
without raw-mode terminal control. That keeps the CLI path scriptable while
letting the interactive TUI feel like a product shell instead of a line-oriented
report.

## Verification expectations

Automated tests should cover launch routing, key-event filtering, reducer
state, no ANSI in plain text output, selected-section rendering, narrow terminal
rendering, and help output. A manual smoke check is still required after local
install refresh because full-screen raw terminal behavior depends on the host
terminal emulator.

Manual check after refreshing the installed binary:

1. Run `rz0` in a new interactive PowerShell terminal.
2. Press down arrow once; selection should advance exactly one section.
3. Hold down arrow; repeat navigation should continue predictably.
4. Press `j`, `k`, Home, and End; navigation should match the help text.
5. Press `h` or `?`; help should toggle without typed input echo.
6. Press `q` or Esc; the TUI should exit and restore the normal prompt.

## Brand and maintainability

TUI visual tokens are centralized in `src/tui_theme.rs` and use the
`BRAND.md` Dossier Navy / Burnished Brass direction. Labels and colors are
secondary to clear text so the dashboard remains usable over SSH, in restricted
terminals, and with color disabled through `NO_COLOR`.

Rendering, app state, input handling, and data shaping are deliberately split:

- `src/tui_dashboard.rs` builds the read-only data model;
- `src/tui_canvas.rs` owns frame, padding, truncation, and line helpers;
- `src/tui_render.rs` renders a resize-safe dashboard shell;
- `src/tui_state.rs` owns navigation/help state transitions;
- `src/tui_app.rs` owns terminal raw-mode lifecycle and event handling;
- `src/tui_theme.rs` owns tokens/status label constants.

Future TUI polish should preserve this separation so the website reference and
real terminal UI can evolve together without making terminal usability depend
on a web layout.
