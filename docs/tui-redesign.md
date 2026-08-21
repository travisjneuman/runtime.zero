# runtime.zero Task-First TUI

Status: active implementation contract for the next public-facing interface.

The active TUI is a calm, task-first workspace built around one question:
**what can I safely do next?** The previous six-section dashboard, persistent
command rail, and repeated status chrome are retired. This document is the
current implementation and acceptance contract, not a claim that every public
release gate has already passed.

## Product direction

The first screen prioritizes the Rust-first AI toolchain capability. It must
make these states legible without requiring the user to understand runtime.zero
architecture:

1. local snapshot is ready, partial, or unavailable;
2. AI/developer tools are installed, missing, duplicated, or ownership-unclear;
3. update review has not run, is running, is ready, or is blocked;
4. one exact supported action can be reviewed, confirmed, cancelled, and
   freshly verified.

The broader inventory, store, module, action, and monitor data remain available
through five workspaces: Home, Toolchain, Software, System, and Diagnostics.
They are navigation destinations, not simultaneous panels on the home screen.

## Layout contract

Every frame has four regions, with no more than two bordered content panels:

```text
runtime.zero / LOCAL CONTROL                         [ready]
local snapshot · 273 software · no action runs without confirmation

HOME   TOOLCHAIN   SOFTWARE   SYSTEM   DIAGNOSTICS

┌ HOME / NEXT STEP ───────────────────────┐ ┌ SELECTED ──────────────────┐
│ one focused list or task summary         │ │ one useful explanation     │
│ selection is obvious without color       │ │ source, status, and action │
└──────────────────────────────────────────┘ └────────────────────────────┘

status message                                                     [? help]
↑↓ move  Enter details  Review action [U]  r refresh  q quit
```

Rules:

- Home shows the next useful action and a small readiness summary; it does not
  repeat the entire architecture or all provider counts.
- Software is the primary list workspace. Rows use a stable name, current
  version, status, and one short source/next-step hint.
- The selected pane explains the selected row. It never repeats the whole row
  list and never claims that a plan executed.
- Toolchain, Software, System, and Diagnostics use the same shell and replace
  only the content, so navigation remains predictable.
- Diagnostics includes the bounded cache observation count, age-threshold
  count, active-use uncertainty, and warning state from the same `cache_review`
  contract as `rz0 cache --dry-run`; it never exposes a second cleanup or
  quarantine action path.
- Diagnostics also shows the bounded quarantine-record count, valid-record
  count, exact-restore availability, and bounded transaction-journal validity /
  action-required counts from the same read-only recovery review contract as
  `rz0 recovery --dry-run`; it does not expose raw host paths or a second
  restore/mutation path. The TUI retains the bounded review-warning count as
  evidence; detailed warning text remains in the CLI/JSON review, never as
  recovery authority.
- Diagnostics combines bounded leftovers evidence with the cache row so the
  shell does not grow a second command rail; it uses the same `leftovers_review`
  contract as `rz0 leftovers --dry-run` and remains report-only.
- The same row states that integrity has no trusted runtime baseline and needs
  an explicit fixture; it does not imply that the TUI performed an integrity
  scan or make a remediation claim.
- Help is a modal overlay, not a permanent panel. Search and confirmation use
  the same modal treatment.
- The footer is one status line plus one short key line. Long prose belongs in
  the selected pane or CLI help.
- Color reinforces a state but never carries its meaning. `[OK]`, `[INFO]`,
  `[PLAN]`, `[WARN]`, and `[BLOCKED]` remain visible when color is disabled.

## Interaction contract

- `Tab`/`Shift+Tab` moves between workspace navigation, primary list, and the
  selected context pane. Focus is announced with text and a marker.
- Arrow keys and `j`/`k` move within the focused list. `Home`/`End` jump to
  boundaries. Mouse wheel scrolls the list under the pointer.
- `Enter` opens the selected explanation. `Esc` closes details, help, search,
  or confirmation before it can quit the application.
- `u` performs an explicit provider availability review and never applies a
  change. Visible `Review action [U]` is the compatibility entry point for the
  exact selected planned update through the shared CLI safety path.
- `r` explicitly refreshes the local snapshot. `m` selects System. `/` opens
  search.
- `?`/`h` opens help. `q` exits and restores the terminal.

## Responsive behavior

- Wide: primary list and selected pane are side by side.
- Standard: the same two surfaces stack vertically, with the selected pane
  below the list and a bounded height.
- Compact: one focused surface is shown at a time; the selected explanation is
  opened with Enter and replaces the list temporarily.
- Very small: show a short safe notice with the minimum size and the
  scriptable `rz0 --no-tui` escape hatch.
- First frame: render the local shell before the full inventory and monitor
  worker completes; show loading, unavailable, empty, and blocked states
  distinctly and never retry a failed worker automatically.

No label may be silently clipped. Text must wrap or truncate with an explicit
ellipsis within the terminal bounds. The frame must remain usable at 58x16 and
must not require a color-capable terminal.

## Safety and evidence

The redesign changes presentation, not authority. The CLI and TUI continue to
share the same typed plan, exact confirmation, executable identity binding,
process host, transaction/receipt, cancellation, and fresh verification path.
The process host also detaches Unix provider children from the controlling
terminal, preventing a provider's own progress UI from corrupting the TUI.
The TUI never invents ownership, turns a fixture into execution input, or
promotes an unsupported provider into a planned action.

## Acceptance gate

- [x] A first-time user can identify the next safe action from the first frame.
- [x] The first frame has no persistent command rail or repeated architecture
  summary.
- [ ] Every workspace renders through the same shell at 58x16, 80x24, 118x30,
  and 160x50 without overflow.
- [x] Home, Toolchain, Software, System, and Diagnostics have distinct useful content and selected
  explanations.
- [ ] Help, search, confirmation, blocked, unavailable, loading, and empty
  states are modal or contextual rather than appended into a crowded footer.
- [ ] Plain and colorized frames have identical text semantics.
- [ ] The same behavior is documented in the CLI/TUI guide and covered by
  buffer, reducer, PTY, and human terminal review.
- [ ] The AI toolchain golden path is demonstrable from discovery through fresh
  verification and receipt without a second TUI-only authority path.
