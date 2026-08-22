# Foundation Readiness Gate

This document records what feature work may rely on in the current foundation
and what still blocks production module or mutation work. It is a maturity gate,
not a production-ready claim.

The current Rust-first presentation checkpoint is `1049103`. Its typed UI
model is validation-bound to the complete five-route set, one model generation,
globally unique record/action identities, redacted bounded text, explicit route
focus semantics, and explicit refresh/failure/recovery transitions. The
scriptable text projection consumes that same model, and foundation help for
`doctor` and `config` is covered by CLI tests. These hardening changes improve
the presentation and contract boundary; they do not authorize blocked
production module execution, network access, or unproven platform mutation
lanes.

For the current source baseline and validation totals, see
[`project-status-and-resumption.md`](project-status-and-resumption.md). For all
remaining 1.0 work, see [`completion-checklist.md`](completion-checklist.md).

## Public contract audit — 2026-08-21

The executable dispatch was traced from `src/main.rs` through launch routing and
`src/lib.rs`. A bare interactive launch enters `src/ui/terminal.rs`; explicit
`--no-tui`, pipe, and redirect launches use the typed `src/ui/text.rs`
projection; `--json` preserves the versioned `foundation_dashboard` contract;
and explicit subcommands remain scriptable CLI paths. The checked top-level
dispatch set is `doctor`, `config`, `apps`, `cache`, `leftovers`, `recovery`,
`restore`, `integrity`, `uninstall`, `completions`, `modules`, `store`, `scan`,
`monitor`, `toolchain`, `report`, `release`, and `updates`.

The audit also traced the shared authority seams: inventory/provider discovery
produces bounded evidence; modules expose lifecycle/status and read-only
process-protocol contracts; updater/uninstall action plans pass through shared
identity, confirmation, cancellation, process-host, transaction, receipt,
verification, and recovery code; and the TUI only renders typed records or
delegates review/execute requests back to those foundation functions. No
presentation surface authorizes a provider, module, process, confirmation,
transaction, receipt, or recovery action.

## Stable-enough implemented surfaces

Current feature work may reuse:

- deterministic launch routing: interactive bare `rz0`, explicit `--tui`, and
  scriptable subcommands/JSON/pipes/redirects/`--no-tui`;
- versioned foundation dashboard, diagnostics, inventory, catalog, monitor,
  findings, plans, registry, receipt, transaction, and support-report schemas;
- one canonical installed-software TUI list with search, filter, sort, refresh,
  details, mouse navigation, update availability, and a native monitor;
- strict local manifest and package-integrity validation, including optional
  complete-file-set and bounded provenance-consistency review;
- dry-run module-install planning and canonical lifecycle transition plans;
- user-local store plan/status, registry/receipt inspection, and explicit Unix
  store scaffolding;
- shared validation, resource, privacy, error, configuration, capability,
  confirmation, cancellation, process, secure-filesystem, artifact-identity,
  registry, transaction, performance, and release-ledger libraries;
- public-test-key signature verification and guarded immutable staging,
  quarantine, restore, and module-transport tests;
- receipt-bound quarantine/restore execution for one exact action, including
  private opened-directory roots, no-replace moves, source/record verification,
  append-only journal snapshots, and filesystem-effect receipts; this is a
  foundation primitive, not a candidate-discovery or public cleanup command.
- built-in bounded inventory reads plus separate first-party module source
  packages;
- live/captured updater evidence, finding-bound plans, serial queue review, and
  the explicitly confirmed core manager-update lane.

These interfaces may still change before 1.0. “Stable enough” means new work
should consume rather than duplicate them; it does not promise semantic-version
compatibility or production support.

## Foundation ownership rule

Cross-module safety and consistency remain foundation-owned:

- schemas, validation, compatibility, migration, and errors;
- trust, signatures, provenance, revocation, package identity, and installed
  state;
- capability, process, filesystem, network, elevation, isolation, cancellation,
  and resource policy;
- findings, action plans, confirmation, transactions, receipts, quarantine,
  rollback, recovery, and post-action verification;
- privacy, configuration, diagnostics, support evidence, performance, and
  release acceptance;
- CLI/JSON/TUI routing and shared platform abstractions.

A module may narrow these contracts. It must not implement a private trust root,
process host, confirmation flow, transaction format, registry, cancellation
engine, or rollback system.

## Safe feature-development boundary

Read-only and synthetic module work may continue when it:

- starts with strict caller-supplied or bounded live evidence;
- declares exact capabilities and preserves default-deny behavior;
- remains useful when a platform source is unavailable;
- produces shared finding/action/support contracts instead of private schemas;
- includes valid, missing, malformed, duplicate, oversized, adversarial,
  permission, timeout, locale, and partial-failure fixtures;
- distinguishes source implementation, compile evidence, runtime evidence, and
  release support;
- keeps protected and unknown data blocked;
- updates CLI/JSON/TUI/docs without implying installation or execution.

## Current write-path exception

The core updater and the narrow leftovers exact-file lane are the only domain
write exceptions. The updater performs a fresh live probe, selects one
finding-bound plan action, obtains exact short-lived confirmation, publishes
local durable evidence, invokes one allowlisted manager path through the
bounded process host, and verifies fresh availability. The leftovers lane
accepts only one explicitly supplied regular file inside the private module
store and invokes the receipt-bound quarantine mover after the same kind of
exact confirmation; it does not discover candidates or recurse.

Do not copy or broaden either lane. Before the updater is production-ready it
still needs:

- opened executable identity bound to the actual spawn;
- platform capability, network, privilege, and process isolation enforcement;
- complete canonical receipt/commit/recovery integration;
- cancellation at every boundary;
- native rollback and tested manual recovery;
- real manager/platform failure, interruption, and power-loss evidence;
- Windows runtime/ACL/reparse containment proof and broader capability policy.

The other current write surface, `store init --yes`, is limited to validated
runtime.zero-owned user-local scaffolding and remains blocked on Windows.
The leftovers lane still needs cross-filesystem, metadata-retention,
platform-bundle, full recovery, and target-native runtime proof.

## Still-blocked product work

The current foundation does not authorize:

- module installation, activation, invocation, repair, migration, upgrade,
  deactivation, or uninstall;
- arbitrary first- or third-party process execution;
- uninstall, broad leftover/cache, permanent-delete, or integrity remediation
  writes; the only narrow exception is the confirmation-bound quarantine of one
  explicitly supplied runtime.zero module-store file;
- broad or domain-discovered quarantine/restore writes. The narrow executor
  requires a caller-provided exact plan and receipt-bound confirmation and does
  not make any module action-ready by itself;
- credential/session/browser-profile/project/backup/unknown-data actions;
- hidden shell/PATH execution, automatic retry, background service, persistence,
  telemetry, or automatic update;
- production keys, package feeds, bootstrap commands, release publication,
  deployment, package submission, or recurring automation.

## Acceptance before a module gains live reads

- [ ] Requirements, privacy classes, sources/roots/managers, and non-goals are
  explicit for every platform.
- [ ] The shared capability vocabulary can express the read without accidental
  mutation/network/elevation authority.
- [ ] Inputs and outputs are strict, versioned, bounded, and privacy-reviewed.
- [ ] Synthetic/adversarial fixtures pass before host access is added.
- [ ] Unsupported/unavailable state remains useful and honest.
- [ ] Final-artifact runtime evidence is planned separately from cross-builds.

## Acceptance before any new mutation lane

- [ ] Exact finding and sealed evidence bind the action.
- [ ] Exact executable/file identity is held through use.
- [ ] Capability, network, privilege, and process containment are enforced.
- [ ] Dry run, write set, state, expiry, and confirmation are exact.
- [ ] Journal, receipt, rollback/quarantine, cancellation, recovery, and fresh
  verification are complete.
- [ ] Every partial/fault/power-loss outcome has disposable-host evidence.
- [ ] CLI/JSON/TUI, accessibility, privacy, performance, support, and platform
  acceptance cells pass.
- [ ] The requested mutation and any external action have current approval.

## Current handoff outcome

The foundation is ready for continued bounded read-only/synthetic domain work
and for hardening the existing updater exception. It is **not** ready for broad
module execution or another write domain. The next highest-value dependency is
to close updater/process/filesystem/transaction production gaps, then bind
uninstall reviews into the same shared pipeline without adding a parallel
security model.
