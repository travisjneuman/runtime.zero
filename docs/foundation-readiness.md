# Foundation Readiness Gate

This document records what feature work may rely on in the current foundation
and what still blocks production module or mutation work. It is a maturity gate,
not a production-ready claim.

For the current source baseline and validation totals, see
[`project-status-and-resumption.md`](project-status-and-resumption.md). For all
remaining 1.0 work, see [`completion-checklist.md`](completion-checklist.md).

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

The core updater is the only domain write exception. It performs a fresh live
probe, selects one finding-bound plan action, obtains exact short-lived
confirmation, publishes local durable evidence, invokes one allowlisted manager
path through the bounded process host, and verifies fresh availability.

Do not copy or broaden this lane. Before it is production-ready it still needs:

- opened executable identity bound to the actual spawn;
- platform capability, network, privilege, and process isolation enforcement;
- complete canonical receipt/commit/recovery integration;
- cancellation at every boundary;
- native rollback and tested manual recovery;
- real manager/platform failure, interruption, and power-loss evidence;
- Windows runtime/ACL/reparse containment proof and broader capability policy.

The other current write surface, `store init --yes`, is limited to validated
runtime.zero-owned user-local scaffolding and remains blocked on Windows.

## Still-blocked product work

The current foundation does not authorize:

- module installation, activation, invocation, repair, migration, upgrade,
  deactivation, or uninstall;
- arbitrary first- or third-party process execution;
- uninstall, leftover, cache, permanent-delete, or integrity remediation writes;
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
