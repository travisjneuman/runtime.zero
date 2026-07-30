# Roadmap

The roadmap is ordered by safety and contract dependency. A checked item means
the documented source/contract exists; it does not imply a release, installation
path, production support, or permission to cross a later gate. Windows, macOS,
Linux, and every frozen 1.0 module family are equal release requirements; see
[`production-readiness.md`](production-readiness.md).

## Phase 1 — foundation baseline (complete)

- [x] Public Rust CLI, safety/security/contribution docs, brand, and static site.
- [x] Read-only Ratatui/Crossterm dashboard with text/JSON fallbacks.
- [x] Module manifest validation and local SHA-256 package integrity.
- [x] Dry-run install planning and user-local store/registry/receipt contracts.
- [x] Explicit store scaffolding as the only foundation write surface.

## Phase 2 — inventory contracts and primitives (implemented; runtime proof remains)

- [x] Versioned inventory JSON and privacy/no-write fields.
- [x] Owned strict deserialization, exact structural/cross-reference/summary
  validation, bounded collections, and a separate private-for-export gate.
- [x] Valid, duplicate, missing, malformed, invalid-entry, and
  unsupported-platform fixtures.
- [x] Bounded process-PATH normalization on Windows/macOS/Linux.
- [x] Read-only Windows User/Machine PATH registry adapters.
- [x] Allowlisted known-executable discovery without recursive scans.
- [x] Opt-in exact-path Unix version probes with no shell, cleared environment,
  shared bounded drains/descriptor audit/process-group teardown, and atomic
  deadline; Windows fails closed pending race-free production containment.
- [x] Structured source status, duration, warnings, and generic events.
- [x] Report-local path redaction for share-oriented output.
- [x] Opt-in normalized Windows application registry evidence.
- [x] Opt-in bounded macOS `.app` and Linux XDG desktop-entry evidence.
- [ ] Real Windows runtime smoke for persisted PATH, registry views, apps,
  version-probe timeout, and redaction.
- [ ] Package-manager listing adapters; intentionally deferred until each
  manager's version/locale/network/source-agreement behavior is proven safe.

See [`inventory-schema.md`](inventory-schema.md).

## Phase 3 — first-party inventory module (source implementation complete)

- [x] Separate `modules/inventory/` workspace package and `rz0-inventory` binary.
- [x] Deterministic text/JSON output and fixture support.
- [x] Read-only TUI slot/command preview without execution.
- [x] Planned first-party manifest for development validation.
- [ ] Signed immutable artifact, package integrity manifest, installation,
  activation, and core execution; blocked by the trust gate.

## Phase 4 — updater planning

- [x] Public plan-first/no-surprise-install design contract.
- [x] Fixture-only schema-1 update-plan model and fail-closed policy tests.
- [x] Separate installed/manager-owned synthetic update finding classifier over
  the shared path-free finding contract; no live adapter or execution.
- [ ] Installed-only availability adapters after explicit network review.
- [ ] Execution remains blocked until trust, transaction, receipt, rollback, and
  confirmation gates are complete.

## Phase 5 — uninstall, leftovers, quarantine, and restore

- [x] Manager-native priority, risk categories, blocked data classes, and
  quarantine/restore design contract.
- [x] Fixture-only uninstall/quarantine/blocked-data plan schemas and policy tests.
- [x] Test-only temporary-root transaction/quarantine/restore simulation with
  verified-copy-before-remove, conflict refusal, failure injection, and receipts.
- [x] Separate synthetic uninstall, leftovers, and cache classifiers requiring
  manager ownership or exact runtime-owned evidence and preserving protected/
  unknown blocking; no live adapter or execution.
- [ ] Platform-specific manager, ownership, ACL, reparse/symlink, locked-file,
  cross-filesystem, and partial-failure proof.
- [ ] Mutation remains blocked pending explicit approval.

See [`action-planning.md`](action-planning.md).

## Phase 6 — interactive UX and site

- [x] Terminal review flow, focus regions, preview-only command rail, and
  responsive layout tiers.
- [x] Inventory module posture visible without implying installation.
- [ ] Manual Windows Terminal/PowerShell TUI smoke and installed-binary refresh.
- [ ] Linux/macOS terminal-emulator accessibility/restore smoke.
- [ ] Website parity and final brand-asset pass after explicit production/site
  approval; source edits may trigger the connected deployment.

## Phase 7 — platform, trust, and distribution gates

- [x] Generic process-PATH inventory compiles for supported platform families.
- [x] Public module threat model, capability, signing, isolation, transaction,
  revocation, and staged-approval design.
- [x] Bounded macOS application-bundle and Linux desktop-entry adapters.
- [x] Separate synthetic security/integrity digest classifier with report-only
  mismatch posture and no remediation claim.
- [ ] macOS/Linux package-manager, service, and persistence inventory adapters.
- [ ] Real Linux application runtime and Windows/full cross-platform compatibility matrix.
- [x] Deterministic target-filtered SPDX 2.3 and deduplicated package license/
  notice evidence generation bound to the exact final binary and artifact
  manifest.
- [ ] Final per-target legal/license and artifact reproducibility review; native
  generation is local evidence, not release approval.
- [x] Shared foundation capability vocabulary and exact manifest partition/
  protocol/action list validators with disjoint schema subsets; validation grants
  no authority.
- [x] Shared typed machine-error vocabulary with redacted-detail defaults and no
  schema-1 automatic retry; module protocol rejects free-form error codes.
- [x] Shared resource contract for artifact/document/collector/probe/process,
  redaction, diagnostics, inventory, and finding ceilings, consumed across
  foundation and modules.
- [x] Shared path-free finding classification contract with exact producer/
  category binding, protected-data blocking, ownership/disposition policy,
  evidence digests, deterministic IDs, and no action authority.
- [x] Bounded privacy contract using domain-separated report-local tokens without
  retaining raw strings; inventory paths are redacted by default.
- [x] Immutable schema-1 configuration defaults enforce privacy, offline/network-
  deny, disabled execution/automation, one-process concurrency, dry-run,
  confirmation, quarantine, and no implicit lifecycle work; diagnostics bind the
  canonical configuration digest.
- [x] Strict privacy-safe text/JSON foundation diagnostics with exact typed checks
  and no host, user, current-directory, environment-value, or raw-path output.
- [x] Add a deterministic foundation support-report contract and separate
  stdin/stdout report/export source module that omit raw inputs and authority;
  signed lifecycle/core integration/final-artifact platform proof remain open.
- [x] Allocation-free validation contract for canonical IDs, versions, lowercase
  hashes, evidence references, and platform-neutral relative paths.
- [x] Require every action plan to bind the finding contract, sealed report ID,
  report digest, and per-action finding ID; deterministically bind plan/write-set
  digests into five-minute exact confirmation and single-use consumption.
- [x] Unix held-directory-relative no-follow create/open/lock/sync/no-replace/
  atomic-replace operations consumed by store, journal, and commit coordination.
- [x] Implement compile-checked Windows `NtCreateFile` root-relative child
  create/open/lock/rename/unlink operations without path-based emulation.
- [x] Add compile-checked exact-owner and bounded DACL inspection that accepts
  allow ACEs only for the user, SYSTEM, or Administrators and rejects unknown
  ACE shapes/principals.
- [ ] Prove Windows owner/DACL privacy, inherited ACLs, reparse/File-ID behavior,
  atomicity, and directory flush on real client/server filesystems.
- [x] Read-only manifest permission schema with default-versus-explicit grants.
- [x] Test-key-only detached Ed25519 signature contract and strict verification.
- [x] Fixture-only immutable staging plan plus atomic temporary-root publication
  simulation bound to successful test-key verification.
- [x] Fixture-only first-party inventory invocation/not-executed response
  protocol with exact receipt path, least-privilege grants, cleared environment,
  and timeout/I/O ceilings.
- [x] Explicit-feature Cargo test-child transport with bounded JSON framing,
  concurrent output drains, timeout kill/reap, environment/cwd proof, and
  fail-closed fixture tests; no module/core execution.
- [x] Move bounded pipe draining and descriptor/handle/test-containment
  primitives into a shared process-host foundation; Windows handle audit fails
  closed rather than reporting success.
- [x] Native Unix test-helper preflight for observed inheritable descriptors and
  process-group timeout teardown including a sleeping descendant.
- [x] Windows-target test-helper Job Object assignment, kill-on-close, bounded
  active-process count, and timeout tree termination; compile evidence only.
- [x] Canonical schema-1 production execution assessment that enumerates every
  artifact/capability/identity/process/runtime/transaction gate while remaining
  incapable of authorization.
- [x] Cross-platform opened-artifact identity primitive that hashes, identifies,
  revalidates, rewinds, and returns the same bounded file handle without
  execution; Unix traversal uses held root-directory handles and no-follow
  component opens.
- [x] Bounded hash-chained transaction journal state machine with exact write
  intent/verification pairing, non-authorizing recovery decisions, exclusive
  cross-process writer locks, and atomic immutable snapshot publication/recovery.
- [x] Add a tamper-evident commit receipt binding the committed journal head,
  action plan, write set, confirmation consumption, and prior/next installed-
  registry digests.
- [x] Durably consume confirmation, publish commit receipts, retain prior registry
  recovery bytes, and atomically publish a canonical validated registry last.
- [x] Add deterministic fault injection at all eight commit boundaries and a
  fresh interactive approval path for exact interrupted registry-last completion.
- [x] Add an allocation-minimal first-writer-wins cancellation/deadline primitive,
  consume it in guarded process timeout polling, and classify it at every
  synchronized commit-coordinator boundary without automatic rollback/retry.
- [ ] Propagate cancellation through remaining production process/write hosts
  and prove real process/power-loss recovery and rollback on every platform.
- [x] Add a non-authorizing borrow-scoped executable binding: Linux held `/proc`
  descriptor path, Windows deny-write/delete handle lease, and fail-closed macOS.
- [x] Integrate Linux/Windows executable leases through guarded test-host spawn.
- [ ] Adversarially prove Linux/Windows binding in a production contained host,
  implement a reviewed exact macOS spawn primitive, close descriptor/
  handle-inheritance and Windows suspended-create races, obtain real Job Object
  proof, enforce capabilities, and complete sandbox/isolation runtime tests.
- [ ] Signing keys, release artifacts, package publishing, bootstrap, remote
  feeds, third-party modules, deployment automation, and production actions only
  after their separate explicit approvals.

See [`module-trust-and-execution.md`](module-trust-and-execution.md).

## Phase 8 — equal-platform, equal-module production 1.0

- [x] Define Windows 11/10/8.1/8/7 and Server 2008-through-2025 compatibility,
  macOS/Linux current-plus-three starting matrices, exhaustive shell/terminal
  research lanes, artifact-only host policy, manager order, architecture
  expansion, and no-paid-signing GitHub distribution posture.
- [x] Add a bounded canonical target × seven-module × 12-stage acceptance-ledger
  contract with deterministic IDs and blocked-only schema-1 release posture.
- [x] Freeze initial final-artifact command ceilings and a strict bounded
  performance evidence schema for version/doctor/scan/dashboard text/JSON paths.
- [ ] Freeze the exact RC target snapshot and target-specific measured/narrower
  budgets, then populate every acceptance ID with reviewed evidence or evidence-
  backed not-applicable.
- [x] Define foundation-owned digest-bound dry-run transitions and exact gates
  for install/activate/invoke/repair/migrate/upgrade/deactivate/uninstall; embed
  the install transition in the core planner.
- [ ] Complete every lifecycle stage for inventory/environment, updater,
  uninstall, leftovers, cache, security/integrity, and report/export on Windows,
  macOS, and Linux.
- [ ] Complete foundation-owned trust, isolation, capabilities, durable
  transactions, module lifecycle, configuration, diagnostics, recovery, and
  compatibility/migrations.
- [x] Add bounded final-artifact PTY/resize/alternate-screen smoke and pass the
  current universal macOS binary through four TERM/dimension cases on ARM64 and
  Rosetta slices.
- [ ] Complete end-to-end/fuzz/race/fault/performance/soak/security/privacy/
  cross-terminal/shell/pipe/Unicode/screen-reader accessibility testing and real
  Windows/Linux/Intel/older-macOS runtime matrices.
- [x] Add local non-publishing native and deterministic universal2 ZIP builders
  plus an unsigned macOS DMG builder that consumes the canonical ZIP, verifies
  exact entries/checksum/content/SBOM/notices, binds honest variable-container
  metadata, and has adversarial preparation tests. Both universal slices execute
  natively/Rosetta on the current Mac.
- [ ] Produce release-reviewed signed/notarized artifacts, SBOM/notices,
  installers, package channels, offline/update/rollback paths, support/incident runbooks,
  and final release evidence.
- [ ] Declare production readiness only after every required acceptance matrix
  cell passes or has an approved tested not-applicable outcome.

See [`production-readiness.md`](production-readiness.md).
