# runtime.zero Rust-first TUI Front-End Reset RFC

Status: implemented product and architecture contract; task-first UI rebuild
complete in source, owner/platform acceptance remains open
Date: 2026-08-22
Repository baseline: task-first source cutover on `main`; verify the current
published commit with `git rev-parse origin/main`
Scope: the Rust terminal UI and its durable product/architecture boundary

This RFC is the reviewable contract for rebuilding the runtime.zero terminal
front end. It settled the product model and first vertical-slice boundary
before source replacement; the implementation and source-cutover record below
now describe the resulting Rust-first presentation.

## 1. Decision summary

runtime.zero will use an evidence-led operator console. Its dominant mental
model is a calm, task-oriented control surface for answering:

> What is the current evidence, what needs attention, and what is the next
> safe step?

The first screen is `Home`. It is an attention queue, not a dashboard of
all available metrics. Within one frame, a first-time operator must be able
to tell whether the local snapshot is ready, whether anything is blocked or
requires review, and how to open the next evidence-backed detail. The screen
must never imply that an action ran merely because an action is available to
review.

The rebuild has these non-negotiable decisions:

1. The foundation owns evidence, action authority, lifecycle semantics,
   confirmation, cancellation, process hosting, transactions, receipts,
   recovery, and JSON/text contracts.
2. The UI owns navigation, focus, presentation state, layout, rendering, and
   terminal lifecycle only.
3. Modules contribute typed, bounded, path-redacted data and references to
   foundation-owned actions. They do not contribute widgets, global chrome,
   key bindings, overlays, lifecycle state machines, confirmation logic, or
   execution paths.
4. Home, Inventory, Evidence, Plan Review, Confirmation, and Activity are
   task states, not a navigation forest. Module records remain content inside
   Inventory and the Home attention queue.
5. The first implementation is a narrow read-only vertical slice. It may show
   a real action-plan boundary and confirmation requirements, but it does not
   create a second apply or recovery authority.
6. The new task-first TUI is the sole active interactive launch target after
   source, buffer, and PTY parity. Human terminal, accessibility, platform, and owner
   acceptance remain separate gates and are not inferred from source tests.
7. The visual direction is `Dossier Queue`: quiet dossier typography, one
   focused attention list, one evidence pane, restrained borders, and
   Burnished Brass used as a semantic accent rather than dashboard decoration.

This RFC does not claim production readiness or human acceptance on terminals
or platforms whose evidence is still outstanding.

## 2. Scope, preservation, and non-goals

### In scope

- product mental model and first-screen behavior;
- stable top-level information architecture;
- typed module-to-UI contribution contract;
- Rust ownership boundaries for the new UI package;
- resource and job state semantics;
- keyboard, mouse, focus, overlay, search, and confirmation behavior;
- terminal, color, accessibility, SSH/tmux, and platform acceptance targets;
- migration boundary, vertical-slice order, rollback, and acceptance gates;
- implementable terminal visual directions.

### Preserved foundation

The new UI is a consumer of these existing contracts. It must not duplicate or
reinterpret them:

- CLI routing, command names, help, exit codes, and machine-readable schemas;
- read-only inventory, diagnostics, monitor, report, store, registry, and
  module-status evidence;
- provider identity, toolchain/update review, findings, and action plans;
- action disposition, capability/risk classification, and exact target binding;
- short-lived exact confirmation and single-use consumption;
- typed cancellation and first-reason/monotonic cancellation semantics;
- bounded process-host transport, identity-to-spawn binding, and containment;
- transaction journals, write intents, verified effects, commit receipts, and
  recovery assessment;
- fresh post-action verification and the distinction between observed,
  planned, confirmed, running, committed, cancelled, and recovery-required;
- public-safe redaction and ANSI-free JSON/text contracts.

### Explicit non-goals

This work does not create a web GUI, a non-Rust frontend, a new provider,
module resource, deployment, release automation, recurring job, or provider
account change. It does not install dependencies, rewrite the CLI, broaden
module execution, or make a production-readiness claim.

## 3. Baseline and design problem

The current canonical checkout was clean on `main` at the baseline named above,
aligned with `origin/main` after a fetch and fast-forward pull. The current
presentation boundary is spread across:

| Existing area | Current responsibility | Reset treatment |
| --- | --- | --- |
| `src/tui_dashboard.rs` | evidence collection, derived rows, user-facing copy, previews, update view state | disposable presentation aggregate; preserve its foundation inputs and audit consumers before removal |
| `src/tui_app.rs` | raw mode, alternate screen, event polling, workers, cancellation, update orchestration, restoration | replace with the new terminal/controller boundary after parity gates |
| `src/tui_state.rs` | global focus, navigation, overlays, search, confirmation input, shortcuts | replace with typed UI state and messages; confirmation validation stays foundation-owned |
| `src/tui_ratatui.rs` | interactive Ratatui layout and widgets | replace with pure screen/widget projections |
| `src/tui_render.rs` | scriptable text rendering and text-shell copy | preserve stable CLI/text semantics; split presentation only after consumer audit |
| `src/tui_layout.rs`, `src/tui_canvas.rs`, `src/tui_*_support.rs` | layout, truncation, style, selection helpers | replace only where the new UI has equivalent bounded behavior |
| `src/tui_theme.rs` | current semantic labels and color helpers | retain the label grammar; move colors into the new theme boundary |
| `src/launch_routing.rs`, `src/color_mode.rs` | interactive/non-interactive routing and color policy | preserved; no TUI redesign may weaken these contracts |

The problem is architectural, not only visual. A generic row model currently
mixes unrelated domains and makes every future module compete for shared
chrome, focus, status copy, and preview space. A loading header can also
coexist with a ready primary row because data and presentation state are not
one coherent projection. The new model must publish a coherent snapshot and
render every state from that projection.

## 4. Product model and first-screen goal

### Dominant mental model: evidence-led operator console

The UI is a console for careful system stewardship. It is not an app launcher,
an installation dashboard, a live telemetry wall, or a module marketplace.
The operator moves through evidence-backed records and reviews a safe next
step. The UI should feel like a well-indexed dossier: quiet, legible, and
deliberate.

The mental model has three layers:

1. **Attention** — items that are loading, blocked, require review, or have a
   clearly available read-only next step.
2. **Evidence** — the selected record's status, source, scope, freshness,
   identity, and relevant foundation references.
3. **Authority boundary** — if an action exists, the selected detail explains
   its disposition and exact review/confirmation requirements. It never turns
   a display affordance into permission.

### First-screen goal

The first frame must answer these questions without requiring architectural
knowledge:

- Is the local snapshot ready, still loading, unavailable, empty, or blocked?
- What one or two items need operator attention?
- What is the safest next inspection or review step?
- If an action is reviewable, what evidence must be inspected before
  confirmation?

The first frame does not attempt to show every module, provider, process,
metric, cache count, or journal field. Detail remains reachable through the
selected record and the scriptable CLI/JSON surfaces.

### Coherent first-frame invariant

`Home` receives one immutable typed projection per render generation.
The projection has one load state and one evidence timestamp/generation. A
row cannot say `ready` while its owning snapshot is still `loading`. If a
worker result is stale, cancelled, disconnected, or invalid, it cannot patch
the current projection. A failed load is `unavailable`; a successful zero-row
load is `empty`; neither is rendered as `ready`.

## 5. Stable information architecture

The typed foundation model retains stable evidence categories, but the
interactive shell presents them through one bounded task flow:

| Task state | Product question | Typical contents |
| --- | --- | --- |
| `Home` | What needs attention and what is the safest next action? | readiness, attention queue, selected explanation |
| `Inventory` | What evidence exists? | searchable foundation records, source/provider identity, module content |
| `Evidence` | What facts support this item? | bounded details, freshness, redaction, disposition |
| `Plan Review` | What exactly is proposed or blocked? | action plan, digests, risk/capabilities, authority boundary |
| `Confirmation` | What explicit decision is required? | short-lived foundation challenge, exact phrase, rollback/recovery posture |
| `Activity` | What is running or already evidenced? | progress, cancellation, receipt, verification, failure, recovery-required |

Navigation remains stable as the module catalog grows. A module can add a
record kind, evidence fields, or a foundation-registered detail section, but it
cannot add a sixth global destination. A future module that needs a new
workflow must first fit its data into one of these questions or propose a
foundation-level architecture change.

The shell has one task surface, one selected-detail or authority surface, and
one status/key footer. It does not contain a route bar, numeric shortcuts,
permanent command rail, metric card grid, or repeated copies of the same status.

## 6. Typed module-to-UI contribution contract

The module boundary is data-only. The foundation turns validated module
evidence into a bounded UI contribution; the UI registry validates and indexes
it. The following Rust-shaped contract is normative design pseudocode for the
first implementation, not a source addition in this phase.

```rust
pub struct ModuleUiContribution {
    pub schema_version: u16,
    pub module_id: ModuleId,
    pub display_name: BoundedText,
    pub posture: ModulePosture,
    pub records: Vec<UiRecord>,
    pub detail_sections: Vec<UiDetailSection>,
    pub action_refs: Vec<UiActionRef>,
}

pub enum ModulePosture {
    InstalledInactive,
    EnabledReadOnly,
    Staged,
    Degraded,
    Unavailable,
    Blocked,
}

pub struct UiRecord {
    pub record_id: UiRecordId,
    pub kind: UiRecordKind,
    pub title: BoundedText,
    pub summary: BoundedText,
    pub state: UiRecordState,
    pub evidence: Vec<EvidenceRef>,
    pub action_refs: Vec<ActionRef>,
    pub search_terms: SearchTerms,
}

pub struct UiActionRef {
    pub action_id: ActionId,
    pub disposition: ActionDisposition,
    pub review: ActionReviewSummary,
}

pub struct EvidenceRef {
    pub source: EvidenceSource,
    pub reference_id: BoundedId,
    pub freshness: Freshness,
    pub redaction: RedactionState,
}
```

Normative properties:

- `ModuleId`, `UiRecordId`, `ActionId`, and evidence references are typed,
  stable, bounded identifiers. They are not arbitrary route strings.
- `BoundedText` rejects ANSI, control characters, unbounded payloads, raw
  host paths, secrets, credentials, and private session data at the adapter
  boundary. Public display text is already redacted; the renderer does not
  discover or redact paths as a last-minute security mechanism.
- `UiRecordState` and `ActionDisposition` come from validated foundation
  evidence. A module cannot claim `succeeded`, `confirmed`, `committed`, or
  `active` by choosing a label.
- `action_refs` are references only. They contain no shell command, callback,
  executable path, process handle, confirmation comparator, write set, or
  worker closure. The UI sends `UiIntent::ReviewAction(ActionId)` to the one
  foundation coordinator, which revalidates the current action and evidence.
- Detail sections use an allowlisted value vocabulary (`Text`, `Count`,
  `Version`, `Status`, `Digest`, `Timestamp`, `Reference`, and bounded table
  rows). A module cannot supply a Ratatui `Widget`, `Block`, `Style`, `Color`,
  arbitrary `Span`, layout coordinates, or terminal escape sequence.
- The foundation registry owns ordering, quotas, duplicate detection, semantic
  labels, redaction checks, and mapping of contribution data to the five
  routes. Contributions that fail validation become `degraded` or
  `unavailable` evidence; they do not partially render as trusted content.
- All global chrome, route names, footer keys, overlays, focus order, search
  semantics, confirmation screens, and job/recovery indicators are
  foundation/UI-owned. Modules cannot override them.

### Authority direction

```text
foundation evidence/action coordinator
        │ validated typed projection
        ▼
module UI adapter ──> UiRegistry ──> Model ──> Messages/State ──> Screens/Widgets
        │                                                │
        └──── action/evidence references only ◄──────────┘
```

There is one execution authority and one confirmation authority. The TUI may
request a review, submit user input to an already-issued foundation challenge,
or request cancellation through the typed coordinator. It cannot execute a
provider, invoke a module, create a transaction, publish a receipt, complete
recovery, or infer success from a process exit.

## 7. State contract

The UI must distinguish view/resource state from job/action state. They may be
shown together, but they are not one enum and cannot overwrite one another.

### Resource/view state

```rust
pub enum ViewState<T> {
    Loading { generation: u64 },
    Ready { value: T, generation: u64 },
    Unavailable { reason: UiError, generation: u64 },
    Empty { generation: u64 },
    Blocked { reason: BlockReason, generation: u64 },
}
```

Required semantics:

| State | Meaning | Allowed UI behavior |
| --- | --- | --- |
| `loading` | no current usable result for this view generation | show progress copy and a cancel/quit path; never present stale rows as current |
| `ready` | validated evidence exists and has one coherent generation | show records and allowed read-only review affordances |
| `unavailable` | collection or provider failed without a safe usable result | show bounded reason and explicit user refresh; no automatic retry |
| `empty` | collection succeeded and contains zero records | say that the result is empty; do not say loading or unavailable |
| `blocked` | policy, capability, identity, platform, or evidence gate prevents the view/action | show the blocking condition and its next evidence step; no bypass affordance |

### Job/action state

```rust
pub enum JobState {
    Idle,
    Running { job_id: JobId, phase: JobPhase },
    Succeeded { receipt: ReceiptRef, verification: VerificationRef },
    Cancelled { job_id: JobId, reason: CancellationReason },
    Recovery { transaction: TransactionRef, decision: RecoveryDecision },
}
```

`running` exists only after the foundation coordinator accepts a typed,
authorized operation. `succeeded` requires the foundation's receipt and fresh
verification references; a green process exit is not enough. `cancelled` is a
typed outcome and is not silently converted to failure or retried. `recovery`
means outcome/evidence requires read-only operator review; it never means the
UI may repair, rerun, roll back, or complete a journal.

### State transition rules

- Worker messages are generation- and request-bound. Late messages are
  discarded, not merged.
- The UI may request `Refresh`, but it cannot schedule or automatically retry
  collection or actions.
- A view can move `loading|refreshing -> ready|unavailable|empty|blocked` only through a
  validated foundation result.
- A job can move `idle -> running -> verified|cancelled|recovery-required|failed` only through
  the foundation coordinator. A UI close, resize, or redraw cannot manufacture
  a terminal job state.
- A `recovery-required` state is sticky until a fresh foundation assessment
  changes it; leaving Activity does not clear evidence.
- Confirmation input is transient UI text. The foundation owns challenge
  issuance, expiry, exact matching, single-use consumption, and binding to the
  action/plan/write-set digests.

## 8. Rust module boundaries

The new UI should be introduced under a Rust-first `src/ui/` boundary. These
files are the planned ownership seams; they are not created in phase 1.

| Boundary | Owns | Must not own |
| --- | --- | --- |
| `src/ui/model.rs` | immutable typed model, route projections, records, evidence refs, bounded display values, semantic status | filesystem/network/process access, action execution, arbitrary strings as authority |
| `src/ui/messages.rs` | `UiEvent`, `UiIntent`, worker result envelopes, request/generation IDs | module-specific keymaps, direct callbacks, untyped event buses |
| `src/ui/state.rs` | task page, focus, selection, search text, confirmation input, view/job display state | lifecycle transition authority, plan construction, confirmation validation, worker spawning |
| `src/ui/layout.rs` | size-aware named regions and responsive layout plans | content-specific coordinate guesses, module-provided geometry |
| `src/ui/widgets.rs` | bounded reusable Ratatui widgets from explicit inputs; labels, lists, detail rows, status banners | foundation reads/writes, module callbacks, raw terminal control |
| `src/ui/widgets.rs` | Home, Inventory, Evidence, Review, Confirmation, and Activity compositions | new global routes, provider execution, private module chrome |
| `src/ui/theme.rs` | semantic roles, Dossier Navy/Burnished Brass palette, no-color fallback, focus/status styles | raw hex values in domain modules, color-only meaning |
| `src/ui/task_terminal.rs` | raw mode, alternate screen, cursor/mouse capture, resize, event polling, panic restoration, PTY lifecycle | data collection, action plans, confirmation, recovery, process-host execution |
| `src/ui/testkit.rs` | deterministic model fixtures, event traces, buffer snapshots, text semantics, terminal-size and color matrix helpers | real provider invocation, real mutation, credential/session access |
| `src/ui/foundation_adapter.rs` | map validated CLI/foundation reports into UI model and map typed intents back to one coordinator | a second provider/action/transaction/recovery implementation |
| `src/ui/mod.rs` | narrow composition and public integration boundary | broad re-export of module internals or authority contracts |

The controller loop may use a TEA-like `update(message) -> state/effects` shape
or an equally explicit architecture. Effects must be typed and centralized;
modules do not receive an effect executor.

## 9. Visual directions

All directions use terminal-native composition, semantic labels, and the
canonical Dossier Navy / Burnished Brass palette from `BRAND.md`. Red appears
only for real danger, error, or blocked unsafe behavior. No direction uses
neon, red security branding, decorative metrics, emoji, or a permanent command
rail.

### Direction A — Dossier Queue (recommended)

```text
runtime.zero   OVERVIEW                                    snapshot  READY
──────────────────────────────────────────────────────────────────────────────
Overview   Explore   Review   Activity   Modules

ATTENTION  2 items need review                              [local evidence]
> [PLAN]  Rust toolchain update review                      1 of 2
  [WARN]  Module registry needs inspection                   installed-inactive
  [OK]    Local snapshot                                    273 records

┌ selected evidence ───────────────────────────┐ ┌ next safe step ──────────┐
│ Rust toolchain update review                  │ │ Review provider evidence │
│ provider   native / observed                  │ │ target   rust toolchain   │
│ state      planned · not confirmed            │ │ action   inspect plan    │
│ evidence   fresh local catalog                │ │ [c] review   [Enter] open│
└───────────────────────────────────────────────┘ └──────────────────────────┘
status: read-only review · no action has run          Tab focus · ? help · q quit
```

Implementation: a two-column `LayoutPlan` at standard/wide sizes, one primary
attention list and one selected-detail pane. At compact sizes, the list and
detail replace one another. The header is two quiet lines, not a card grid.
Brass marks the selected row and plan/review labels; sage, amber, blue-gray,
and red are semantic only. This direction best supports the five-second
first-screen goal and future modules because new records join the queue by
typed status instead of adding panels.

### Direction B — Evidence Ledger

```text
runtime.zero / REVIEW                              local snapshot · ready

01  [WARN] registry evidence       installed-inactive   09:42
02  [PLAN] toolchain review        reviewable            09:41
03  [INFO] system snapshot         fresh                 09:40
04  [OK]   inventory               273 records           09:40

──────────────────────────────────────────────────────────────────────────────
02 · toolchain review
source  native provider observation
state   planned; confirmation has not been requested
next    open the foundation-owned action plan

↑↓ select · Enter details · / search · c confirm · Esc back · q quit
```

Implementation: a dense but calm single-column `Table`/`Paragraph` ledger,
with a bounded detail drawer below. It is strong for Activity and Review and
works well over SSH/tmux, but it makes Overview less immediately spatial and
can become chronology-heavy as modules grow.

### Direction C — Quiet Index

```text
runtime.zero  /  OVERVIEW  /  2 attention items

INDEX                         DOSSIER
Overview                      Rust toolchain update review
Explore                       ──────────────────────────────
Review                        disposition   planned
Activity                      provider      native
Modules                       evidence      fresh local catalog
                              authority     foundation action plan
> attention                   next          review; do not execute
  local snapshot              [Enter] open evidence

──────────────────────────────────────────────────────────────────────────────
read-only · [OK] labels remain visible without color · ? help · q quit
```

Implementation: a narrow index rail and a generous text dossier, with only
one selected marker and a single rule. It has the lowest visual noise and the
best screen-reader reading order, but it under-emphasizes a growing attention
queue and spends horizontal space on navigation.

### Recommendation

Adopt Direction A. It preserves the product's safety-led dossier character,
gives the first frame a clear next step, and scales through typed records rather
than more global panels. Borrow Direction B's ledger treatment for `Activity`
and Direction C's quiet rule/reading order for compact and no-color modes.
This hybrid is an implementation detail inside the recommended direction, not
permission for modules to inject layouts.

## 10. Interaction contract

The keymap is global and foundation/UI-owned. A module may declare that a
record supports an action; it may not add or shadow a key.

### Keyboard

| Key | Behavior | Safety boundary |
| --- | --- | --- |
| `q` | quit after requesting cancellation of UI-owned work | does not cancel an external effect by pretending it rolled back; terminal is restored regardless |
| `Esc` | close help/search, cancel confirmation, or move to the parent task state | never confirms, retries, or clears recovery evidence |
| `Tab` / `Shift+Tab` | move through task queue, selected detail, and controls | focus only |
| arrows / `j` / `k` | move within the focused list or scroll detail | bounded selection; no action execution |
| `Home` / `End` | move to the first/last visible item | bounded selection |
| `Enter` / `Space` | open detail or foundation-owned review surface | read-only until a valid foundation action flow says otherwise |
| `i` / `h` / `a` | open Inventory, Home, or Activity | task navigation only; modules cannot add destinations |
| `/` | open search for the current searchable record set | local filtering only unless a foundation review explicitly says otherwise |
| `r` | request an explicit refresh | no automatic retry; stale generations cannot win |
| `?` | open help overlay | help is a modal, not permanent chrome |
| `u` | request provider availability review where the foundation exposes it | read-only network metadata review; never apply |
| `c` | request the foundation confirmation challenge for the selected reviewable action | never bypasses plan, identity, confirmation, or receipt gates |
| `Ctrl+C` | send typed cancellation to the active coordinator and show its outcome | cancellation is not rollback and never authorizes a rerun |

The footer shows only keys valid for the current state. A key is not shown as
available unless the current model advertises the corresponding typed intent.

### Mouse and focus

- A click on a record selects it; a click on the selected detail opens the
  next task state when one is available.
- Mouse wheel scrolls the list or detail under the pointer by a bounded amount.
- The focused region is visible without color using a `>` marker, a text label,
  or a reversed/bold fallback. Focus order is deterministic and matches the
  reading order: task queue, selected detail, controls.
- Pointer coordinates are interpreted from the current `LayoutPlan`; widgets
  do not guess global coordinates. Unsupported mouse reporting is harmless.
- Mouse cannot submit an exact confirmation phrase accidentally: confirmation
  submission requires the dedicated foundation-owned control and the same
  challenge/expiry checks as keyboard input.

### Overlays

There is one small modal overlay stack owned by the UI shell. Only the top
overlay receives input. The first implementation needs only these overlays:

1. help;
2. search input/results;
3. no action or recovery overlays; those are dedicated task states.

An overlay declares its focus trap, dismissal key, semantic title, and bounded
content. Modules supply data sections to an overlay; they cannot create a new
overlay kind or alter dismissal/confirmation rules.

### Search

Search is an explicit, local, read-only filter over the current typed record
set. It searches foundation-approved title, summary, module, provider, status,
and bounded search terms. It does not perform network discovery, invoke a
provider, read raw paths, or change the underlying snapshot. Empty results are
rendered as `empty`, not `unavailable`. Search text is transient and is not
sent to modules as an execution command.

### Confirmation

The Plan Review task state must display the foundation-provided operation,
target identity, provider, action/plan ID, plan and write-set digests where
safe, capabilities, network/elevation posture, rollback/recovery posture,
expiry, and the exact phrase requirement. It must visibly say that the action
has not run.

The flow is:

```text
record selected
  -> foundation validates current action reference
  -> Plan Review task state
  -> foundation issues/returns exact challenge
  -> transient confirmation input
  -> foundation validates exact phrase and single-use consumption
  -> foundation coordinator owns running/receipt/verification/recovery state
```

The UI never compares the phrase, constructs a plan, consumes a challenge,
spawns a process, writes a journal, publishes a receipt, or treats a callback
as success. Cancellation, expiry, mismatch, stale evidence, and recovery are
explicit text states. `Esc` cancels the confirmation view; it does not cancel
or reverse an already-started external effect.

## 11. Terminal and accessibility acceptance matrix

This matrix is an acceptance target for the new front end, not current evidence
that every cell passes.

| Environment/lane | Target sizes and modes | Required proof |
| --- | --- | --- |
| local macOS terminal | `58x16`, `80x24`, `118x30`, `160x50`; color and no-color | no overflow, stable focus, first-frame state, resize, `q`, panic/normal restoration |
| local Linux terminal | same matrix; UTF-8 and conservative glyph fallback | same shell, semantic labels, no raw path leakage, no terminal corruption |
| Windows Terminal | same matrix; key press/repeat/release behavior | no double navigation, raw-mode/cursor restoration, resize, no-color readability |
| Windows console compatibility lane | compact and standard sizes | fallback borders/glyphs, input behavior, explicit unsupported/degraded message where needed |
| direct SSH | `80x24` and `58x16`, color negotiation off/on | no assumption of local terminal features, no provider progress output corrupting the frame |
| SSH inside tmux | `80x24`, `118x30`, resize pane | correct dimensions, mouse/focus behavior, redraw stability, safe detach/quit |
| `NO_COLOR` / `--color=never` | all automated sizes | text labels carry every state; no color-only focus/status meaning |
| screen-reader/assistive reading | standard and compact | predictable route/list/detail/footer order, explicit labels, no status encoded only by symbols or color |
| non-interactive/piped | no TUI frame | existing launch routing selects deterministic text/JSON; JSON remains ANSI-free and schema-stable |

### Size policy

The migration keeps the current bounded tier contract while the new layout is
implemented:

- below `50x12`: safe notice, minimum-size guidance, and the scriptable
  `rz0 --no-tui` escape hatch;
- `50x12` through compact sizes: one focused surface at a time, no clipped
  controls;
- `72x20` through standard sizes: list and detail stack vertically;
- `110x24` and wider: list and detail may sit side by side;
- `58x16`, `80x24`, `118x30`, and `160x50` are mandatory snapshot/PTY review
  sizes even when the terminal advertises a larger geometry.

No text required for understanding may be silently clipped. Long content wraps
or uses an explicit ellipsis, and the selected detail remains reachable when
the primary list is long.

### Color and semantic rendering

The theme uses semantic roles mapped to Dossier Navy / Burnished Brass: void,
canvas, panel, raised panel, subtle/strong border, bone-white primary text,
blue-gray metadata, brass focus/plan, sage success, amber warning, red danger,
and violet dry-run. Raw color values stay in `ui/theme.rs`.

Every status has a plain label such as `[OK]`, `[INFO]`, `[PLAN]`, `[DRY-RUN]`,
`[WARN]`, `[BLOCKED]`, `[QUARANTINE]`, `[SKIP]`, or `[ERROR]`. Color reinforces
the label; it never replaces it. JSON and redirected text remain ANSI-free.

## 12. Migration boundary and rebuild order

### Phase 1 — this RFC and kickoff (historical; complete)

- preserve the canonical source and current TUI behavior while the contract is
  reviewed;
- record the baseline, product decision, typed contract, visual direction,
  state model, acceptance matrix, and gates;
- do not create `src/ui/`, alter launch routing, or delete current files during
  the kickoff phase;
- validate this document and the unchanged repository without claiming visual
  or production acceptance.

### Phase 2 — typed model and testkit

- add the new Rust UI boundary beside the current presentation code;
- implement typed model/messages/state/layout/test fixtures;
- build a foundation adapter from existing validated reports;
- keep the current TUI as the default launch path;
- prove contribution quotas, redaction, stable IDs, and no widget injection.

### Phase 3 — first vertical slice

The explicit slice is:

```text
Home loading
  -> coherent Home ready projection
  -> select one concrete evidence-backed item
  -> inspect evidence
  -> inspect exact plan and authority
  -> enter dedicated foundation confirmation
  -> observe activity, verified, cancelled, failed, or recovery-required
```

The slice must exercise at least one module/provider-backed record and one
foundation action reference, but it must not apply the action. It must show
`planned`, `blocked`, or `reviewable` evidence without claiming execution.
The slice is complete only when the same UI model renders deterministic
loading, ready, unavailable, empty, and blocked fixtures and the coordinator
boundary is covered by tests.

### Phase 4 — widen by task state

Add Inventory, Evidence, Plan Review, Confirmation, and Activity one at a time.
Each slice must add typed foundation data and tests before adding visual
surface. Activity must consume existing job, cancellation, receipt, and
recovery evidence; it must not create private lifecycle or recovery state
machines. Modules remain content inside Inventory and must not expose
activation/invocation authority that the foundation does not provide.

### Phase 5 — cut over and retire disposable presentation

Only after the vertical slice and destination gates pass:

1. switch the interactive route at the narrow `run_interactive_tui` boundary;
2. preserve `--no-tui`, subcommands, text, JSON, and launch-routing behavior;
3. run a complete consumer inventory for every old `tui_*` symbol;
4. remove presentation-only files in a separate, reviewable commit;
5. retain or move any shared data adapter needed by CLI/text contracts;
6. run the full source, PTY, terminal, accessibility, and platform matrix.

The old shell and new shell may coexist temporarily during migration, but this
is a transition mechanism, not a permanent dual-UI architecture.

## 13. Rollback strategy

- Every phase is one or more small commits with an explicit routing boundary;
  no reset, broad cleanup, force-push, or destructive checkout operation is a
  rollback plan.
- Until the cutover gate passes, the old TUI remains available as the known
  fallback. A failed new slice can be disabled by reverting only its routing
  commit or by restoring the prior launch call in a reviewable commit.
- After cutover, rollback is a Git revert of the new UI/routing commit while
  preserving foundation changes. Do not revert or rewrite safety contracts to
  make the UI compile.
- A runtime cancellation never implies rollback. If an external effect may
  have started, the UI shows `cancelled` or `recovery` from the foundation and
  directs the operator to read-only recovery evidence.
- A malformed or stale contribution is isolated as degraded/unavailable data;
  it does not trigger automatic retries, module removal, or fallback execution.
- Terminal lifecycle rollback is unconditional: panic, error, cancellation,
  and normal quit restore raw mode, cursor visibility, mouse capture, and the
  normal screen. Failure to restore is a UI defect to report, not permission
  to mutate system state.

## 14. Concrete acceptance gates

### Gate A — RFC coherence (phase 1)

- [x] dominant mental model and first-screen goal are explicit;
- [x] the task-first Home → Inventory → Evidence → Plan Review → Confirmation →
  Activity workflow and module-as-content scaling rule are explicit;
- [x] typed contribution contract forbids widget/chrome/keymap/authority
  injection;
- [x] requested Rust boundaries and all required states are defined;
- [x] interaction, terminal, accessibility, SSH/tmux, and platform targets are
  defined;
- [x] three visual directions are implementable and one is recommended;
- [x] no source deletion or route replacement occurs in phase 1;
- [x] vertical slice, migration boundary, rollback, and gates are reviewable.

### Gate B — model/testkit foundation

- [x] `src/ui/model.rs`, `messages.rs`, `state.rs`, `layout.rs`, `theme.rs`,
  `testkit.rs`, and the adapter compile without changing CLI authority;
- [x] fixture contributions with duplicate IDs, ANSI/control text, raw paths,
  oversized fields, invalid action references, and unsupported widgets fail
  closed;
- [x] no module UI code imports terminal lifecycle, process-host, transaction,
  confirmation, or recovery implementation types directly;
- [x] model tests cover all resource and job states and reject stale messages;
- [x] existing JSON/text and launch-routing tests remain unchanged and pass.

### Gate C — vertical slice

- [x] Overview has one coherent loading/ready projection and never mixes
  generations;
- [x] selection, detail, action review, and Esc return work at all mandatory
  sizes in color and no-color modes;
- [x] the selected action shows plan/identity/capability/confirmation/recovery
  evidence and visibly says it has not run;
- [x] no provider/module process starts from the UI test path;
- [x] buffer snapshots, event traces, text-semantic assertions, and a real
  PTY smoke exist for the slice.

### Gate D — human terminal review

- [ ] macOS, Linux, and Windows terminal sessions pass the matrix;
- [ ] SSH and SSH/tmux sessions pass the matrix;
- [ ] no-color and assistive reading review confirms state meaning without
  color, layout guesswork, or clipped required text;
- [ ] `q`, `Esc`, resize, cancellation, panic restoration, and non-interactive
  launch routing are observed in real terminal evidence;
- [ ] review records exact terminal, size, mode, and commit; no visual pass is
  inferred from a Ratatui buffer test.

### Gate E — cutover and retirement

- [x] current CLI contracts, JSON/text output, module lifecycle, provider and
  action authority, confirmation, cancellation, process-host, transactions,
  receipts, and recovery evidence have parity or an explicit unchanged proof;
- [x] the new UI is the only interactive presentation path at the cutover
  commit;
- [x] consumer inventory proves each old presentation file is unused or has a
  documented shared-contract reason to remain;
- [x] old interactive presentation source is removed after source, buffer, and
  PTY parity evidence;
- [ ] owner acceptance is recorded. This gate still does not by itself make
  runtime.zero production-ready; the project release contract remains the
  authority for that decision.

## 15. Implementation status after Gate B–E source cutover

The typed model/testkit foundation, task-first Home-to-Activity flow, explicit
provider-review worker, foundation-owned confirmation/execute delegation, and
semantic outcome states are implemented in `src/ui/`. `task_terminal.rs` is
the sole active interactive controller. The old route shell, screen
composition files, and unused direct terminal controller were retired;
`tui_dashboard.rs` remains only as a bounded foundation snapshot source,
while `src/ui/text.rs` is the typed scriptable projection used by the CLI.

Source and deterministic test evidence cover the required local sizes,
color/no-color semantics, reducer states, stale generations, mouse/keyboard
focus, search, confirmation input, cancellation, recovery, and text/JSON
parity. A real macOS PTY smoke at 80x24 observed loading, ready local evidence,
Overview selection, detail, read-only review, Esc return, and terminal
restoration; a 42x10 PTY observed the bounded terminal-too-small escape. Human
visual review, SSH/tmux, Linux/Windows, screen-reader, panic-restoration, and
owner acceptance remain separate evidence lanes and are not asserted by this
source cutover.

## References

- [`BRAND.md`](../BRAND.md) — canonical Dossier Navy / Burnished Brass rules;
- [`SAFETY.md`](../SAFETY.md) — TUI read-only and foundation safety boundary;
- [`tui.md`](tui.md) — current terminal routing and interaction contract;
- [`tui-redesign.md`](tui-redesign.md) — current task-first implementation
  baseline, retained until this rebuild crosses its cutover gates;
- [`action-planning.md`](action-planning.md) — evidence-to-plan-to-confirmation
  flow;
- [`confirmation-contract.md`](confirmation-contract.md),
  [`cancellation-contract.md`](cancellation-contract.md),
  [`process-host-foundation.md`](process-host-foundation.md),
  [`transaction-journal.md`](transaction-journal.md), and
  [`recovery-guide.md`](recovery-guide.md) — preserved authority contracts;
- [Ratatui layout concepts](https://ratatui.rs/concepts/layout/);
- [Ratatui The Elm Architecture](https://ratatui.rs/concepts/application-patterns/the-elm-architecture/);
- [Ratatui component architecture](https://ratatui.rs/concepts/application-patterns/component-architecture/).
