# Terminal UI guide

`rz0` opens the interactive terminal UI only when stdin and stdout are real
terminals and automation is not detected. Use `rz0 --tui` to request it
explicitly, `rz0 --no-tui` for deterministic text, and `rz0 --json` for the
read-only machine-readable dashboard. Explicit subcommands never launch the
TUI.

The interactive path is a task-first operator console over the same typed
foundation evidence, provider review, action-plan, confirmation, cancellation,
transaction, receipt, verification, and recovery contracts used by the CLI.
The UI owns presentation and terminal lifecycle only; it cannot invent an
action or bypass a foundation gate.

## The operator path

Home answers three questions in one quiet frame: is the local evidence ready,
what needs attention, and what is the single safest next action. The normal
path is deliberately linear:

```text
Home
  -> select a concrete attention or evidence item
  -> Enter: inspect the evidence dossier
  -> Enter: read the exact foundation plan and authority boundary
  -> c: request the short-lived foundation confirmation challenge
  -> type the exact phrase in the dedicated confirmation state
  -> Enter: submit to foundation validation
  -> Activity: see progress, cancellation, verified receipt, or recovery-required
```

`c` is only recognized inside Plan Review. There is no global confirmation
shortcut. Confirmation input is never interpreted as navigation, and Esc
cancels the challenge without implying rollback.

Inventory (`i`) is the searchable evidence index. It contains the current
foundation records, including module-contributed content; modules do not add
tabs, widgets, global key bindings, lifecycle state, or execution authority.
Activity (`a`) is the visible place for provider review, action preparation,
progress, cancellation, receipts, verification, failed outcomes, and
recovery-required evidence. Provider review is explicit (`u`) and has no
automatic retry.

## States

The first frame and every task surface carry a semantic state label. The UI
keeps these distinct:

- `loading`: the first local snapshot has not published;
- `refreshing`: an explicit refresh is collecting a new generation;
- `ready`: validated evidence exists;
- `empty`: collection succeeded and found nothing;
- `unavailable`: collection could not provide evidence;
- `blocked`: foundation policy prevents the next step;
- `failed`: the foundation or provider returned a failed outcome;
- `cancelled`: the operator stopped a job; this is not rollback;
- `verified`: a receipt and fresh verification reference were returned;
- `recovery-required`: read-only recovery evidence needs operator review.

An empty result is never rendered as loading or unavailable. A late worker
result from an older generation cannot replace the current snapshot.

## Layout and terminal behavior

The shell has one header, one task/evidence surface, one selected detail or
authority surface, and one status/control footer. It has no persistent command
rail, numeric route bar, implementation-metric cards, duplicate status cards,
or packed one-line row grid.

```text
runtime.zero / HOME  ready
what needs attention · the safest next action
┌ Next safe actions ──────────────────────┐ ┌ Selected evidence ─────────┐
│ > REVIEW  Rust toolchain plan           │ │ [PLAN] Rust toolchain plan │
│   provider evidence is ready             │ │ source: foundation         │
│   PLAN    local evidence needs review    │ │ next: Enter inspect plan   │
└─────────────────────────────────────────┘ └─────────────────────────────┘
status  local evidence ready · job idle
↑↓ select · Enter inspect · i inventory · a activity · ? help · q quit
```

The layout adapts without changing the workflow:

- below `50x12`: a semantic safe notice plus `rz0 --no-tui` escape hatch;
- compact terminals: queue and detail stack vertically;
- standard and wide terminals: queue and detail sit side by side where space
  permits;
- resize redraws from the same typed state, with no route or selection reset.

The terminal guard enters raw mode and the alternate screen, hides the cursor,
captures mouse input, ignores key-release navigation, and restores terminal
state on normal exit, cancellation, error, and panic unwinding. `NO_COLOR` and
`--color=never` preserve all labels, status words, and focus instructions
without relying on color.

## Controls

- `q`: quit and cancel UI-owned workers;
- `Esc`: close help/search, cancel confirmation, or step back; when a job is
  running it requests cancellation;
- `↑`/`↓` or `j`/`k`: select an item in the focused queue;
- `Tab` / `Shift+Tab`: cycle task queue, detail, and controls focus;
- `Enter`: inspect the selected item or open its exact plan from Evidence;
- `c`: prepare foundation confirmation, only from Plan Review;
- `i`: open Inventory; `h`: return Home; `a`: open Activity;
- `u`: explicitly review provider evidence; `r`: explicitly refresh local
  evidence; `/`: locally search Inventory; `?`: show controls;
- mouse click: select a queue item or open the selected detail; wheel moves the
  bounded queue.

The CLI remains available at every terminal size. `rz0 --no-tui` is the
deterministic text projection of the same typed model, and `rz0 --json` remains
the ANSI-free `foundation_dashboard` contract.

## Foundation boundary

The UI consumes bounded, path-redacted records from `src/ui/foundation_adapter.rs`.
It does not collect providers directly, construct plans, validate confirmation
phrases, execute processes, write transactions, interpret receipts, or decide
recovery. A planned action is re-prepared through the existing foundation
function against fresh evidence before the challenge is displayed.

The active presentation implementation is split into pure model/state/layout/
widget projections and `src/ui/task_terminal.rs`, which owns raw-mode lifecycle,
event translation, cancellation, and delegation to the foundation workers.
The scriptable projection is `src/ui/text.rs`; the old direct terminal module is
not an interactive launch target.

## Validation contract

TUI coverage includes reducer task-flow tests, deterministic Ratatui buffers,
all required sizes (`58x16`, `80x24`, `118x30`, `160x50`), no-color semantic
assertions, loading/refreshing/empty/unavailable/blocked/failed/cancelled/
verified/recovery-required outcomes, mouse bounds, and a PTY smoke. Run the
smallest relevant checks first:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo run --locked -- doctor
cargo run --locked -- scan --dry-run
```

Automated buffers and PTY evidence do not replace human review in real macOS,
Linux, Windows, SSH/tmux, and screen-reader sessions. This project remains
pre-alpha; source/test evidence is reported separately from platform,
accessibility, and owner acceptance.
