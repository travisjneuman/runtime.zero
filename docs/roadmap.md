# Roadmap

The roadmap is ordered by safety and contract dependency. A checked item means
the documented source/contract exists; it does not imply a release, installation
path, production support, or permission to cross a later gate. Windows, macOS,
Linux, and every frozen 1.0 module family are equal release requirements; see
[`production-readiness.md`](production-readiness.md).

> **Reviewed 2026-08-21.** The 2026-07-30 pause was superseded by the
> 2026-08-01 TUI, updater, and native-monitor continuation. Resume with
> [`project-status-and-resumption.md`](project-status-and-resumption.md), which
> records the current implementation boundary, validation totals, known debts,
> and restart sequence. Use [`documentation-index.md`](documentation-index.md)
> for document precedence.

## Product horizon: full system management through modules

The roadmap below is the initial release path, not the terminal product scope.
The end state is a full system-management platform in which every feature or
provider is an independently versioned module that users can install, enable,
configure, disable, update, repair, or uninstall for their own use case. The
foundation owns the shared control plane; modules own domain behavior. The TUI
is the primary interactive workflow and the CLI remains an equally capable
scriptable surface over the same contracts.

The initial seven families — inventory/environment, updater, uninstall,
leftovers, cache, security/integrity, and report/export — are the first release
gate. Later waves can add developer and AI toolchains, additional package/app
providers, services and persistence, storage/data hygiene, network and hardware
management, OS settings, backup/recovery, automation, account/provider
integrations, and explicitly separated remote/fleet modules. A new family does
not inherit support from a similar one: it gets its own provider, platform,
trust, capability, lifecycle, TUI/CLI/JSON, transaction, recovery, and release
evidence cells. See [`engineering-handoff.md`](engineering-handoff.md) for the
full catalog, module contract, state semantics, and shift handoff.

The dependency order is therefore:

1. finish the foundation-owned module store, trust, lifecycle, configuration,
   capability, process, transaction, and recovery platform;
2. bring the initial seven families through equal-platform production evidence;
3. expand source/provider coverage and add new capability modules in explicit
   waves without enlarging the foundation through special cases;
4. consider third-party and remote/fleet ecosystems only after local lifecycle,
   revocation, support, and recovery are production-ready.

## Phase 1 — foundation baseline (implemented; not a production gate)

- [x] Public Rust CLI, safety/security/contribution docs, brand, and static site.
- [x] Interactive Ratatui/Crossterm dashboard with text/JSON fallbacks, visible
  selection/details, mouse-wheel list navigation, and a live system monitor.
- [x] Module manifest validation and local SHA-256 package integrity.
- [x] Dry-run install planning and user-local store/registry/receipt contracts.
- [x] Explicit store scaffolding and confirmation-bound updater manager
  execution as foundation write surfaces.
- [ ] Promote both write surfaces to complete platform runtime evidence; updater
  now has working macOS/Linux manager execution, cancellation-aware execution,
  canonical external-effect receipts, isolated npm execution, and fresh
  post-action verification, but still needs Windows runtime/ACL proof, recovery
  completion, rollback, and disposable-host proof.

## Phase 2 — inventory contracts and primitives (implemented; runtime proof remains)

- [x] Versioned inventory JSON and privacy/no-write fields.
- [x] Owned strict deserialization, exact structural/cross-reference/summary
  validation, bounded collections, and a separate private-for-export gate.
- [x] Valid, duplicate, missing, malformed, invalid-entry, and
  unsupported-platform fixtures.
- [x] Bounded process-PATH normalization on Windows/macOS/Linux.
- [x] Read-only Windows User/Machine PATH registry adapters.
- [x] Allowlisted known-executable discovery without recursive scans.
- [x] Opt-in exact-path version probes with no shell, cleared environment,
  shared bounded process-host drains/containment, and atomic deadline; Windows
  uses the production pre-start Job Object/handle-list path.
- [x] Structured source status, duration, warnings, and generic events.
- [x] Report-local path redaction for share-oriented output.
- [x] Opt-in normalized Windows application registry evidence.
- [x] Opt-in bounded macOS `.app` and Linux XDG desktop-entry evidence.
- [x] Built-in path-free software catalog and live redacted core scan.
- [x] Bounded macOS `Info.plist` versions and Homebrew Cellar/Caskroom metadata
  without manager execution or network access.
- [x] Direct bounded MacPorts/Apple receipt and Linux dpkg/pacman metadata reads,
  source-specific software identifiers, and launchd/systemd/Windows service
  metadata without manager/service-controller execution.
- [x] Direct bounded Flatpak `active/metadata` reads with app ID/architecture/
  branch identity and explicit unsupported action posture.
- [ ] Real Windows runtime smoke for persisted PATH, registry views, apps,
  version-probe timeout, and redaction.
- [ ] Additional package-manager listing adapters; intentionally deferred until
  each manager's version/locale/network/source-agreement behavior is proven safe.

See [`inventory-schema.md`](inventory-schema.md).

## Phase 3 — first-party inventory module (source implementation complete)

- [x] Separate `modules/inventory/` workspace package and `rz0-inventory` binary.
- [x] Deterministic text/JSON output and fixture support.
- [x] Inventory library embedded as the installed core's bounded read-only
  adapter, with live TUI, `rz0 apps`, and `rz0 scan` surfaces.
- [x] Planned first-party lifecycle manifest retained for development validation.
- [ ] Signed immutable lifecycle artifact, activation, and out-of-process module
  execution; blocked by the trust gate and not needed for built-in reads.

## Phase 4 — updater planning

- [x] Public plan-first/no-surprise-install design contract.
- [x] Fixture-only schema-1 update-plan model and fail-closed policy tests.
- [x] Separate installed/manager-owned synthetic update finding classifier over
  the shared path-free finding contract; live availability and manager execution
  are owned by the core updater lane.
- [x] Installed-only availability adapters after explicit network review;
  bounded live Homebrew formula/cask probes are integrated into the TUI and CLI.
- [x] Explicit one-item and interactive serial manager execution with exact
  confirmation, transaction journal, receipt, and post-action verification.
- [x] Homebrew JSON and bounded APT/DNF/Pacman/MacPorts captured-output parsers;
  Winget specifications currently fail closed at parsing; Zypper uses a strict
  XML package-row parser; Snap uses a strict five-column table parser; Flatpak
  uses a strict JSON, ref/commit-bound parser.
  WinGet remains intentionally unavailable because the documented
  `list --upgrade-available` surface is a human table, while documented JSON
  output belongs to `export` and does not carry available-update fields; the
  official client request for `list --json` is closed as not planned. See
  [Microsoft's list command](https://learn.microsoft.com/en-us/windows/package-manager/winget/list),
  [export command](https://learn.microsoft.com/en-us/windows/package-manager/winget/export),
  and the [upstream JSON-output issue](https://github.com/microsoft/winget-cli/issues/4965).
- [x] Provider-driven all-source review for installed system managers, language
  environments, known self-updaters, multiple npm prefixes, and declared app
  metadata, with explicit missing/observed-only/unsupported-source warnings;
  this does not claim universal provider coverage.
- [x] Implement the macOS path identity/digest revalidation binding and
  provider-native manager apply path; Linux direct execution uses and
  revalidates its held `/proc/self/fd` binding.
- [ ] Complete Windows opened-executable identity-to-spawn binding and enforce
  platform capability/network/elevation policy across the full target matrix.
- [x] Reconcile updater journal/receipt publication through the canonical
  external-effect transaction/recovery model with write-intent/outcome evidence
  and deterministic read-only recovery assessment.
- [x] Propagate the caller-owned cancellation token through updater apply-time
  discovery, serial refresh, manager execution, post-action verification, and
  installed-software inventory/tool probes; confirmed Unix execution bridges
  SIGINT through bounded process-group teardown, Windows now has a native
  console-control bridge that is compile-checked for MSVC, and cancellation
  remains fail-closed before receipt publication. Target-native Windows
  runtime and console-event acceptance are still open.
- [ ] Native rollback, Windows runtime/ACL/reparse proof, manager-specific locale/
  source-agreement/offline/runtime proof, manager-specific recovery beyond the
  local journal completion lane, real
  failure/recovery evidence, and equal-platform production acceptance remain.

## Phase 5 — uninstall, leftovers, quarantine, and restore

- [x] Manager-native priority, risk categories, blocked data classes, and
  quarantine/restore design contract.
- [x] Fixture-only uninstall/quarantine/blocked-data plan schemas and policy tests.
- [x] Test-only temporary-root transaction/quarantine/restore simulation with
  verified-copy-before-remove, conflict refusal, failure injection, and receipts.
- [x] Separate uninstall, leftovers, and cache classifiers requiring manager
  ownership or exact runtime-owned evidence and preserving protected/unknown
  blocking; cache has bounded read-only discovery and leftovers additionally
  has one confirmation-bound exact module-store-file quarantine lane, while
  broad mutation adapters remain gated.
- [x] Live path-free installed-software evidence is converted into the shared
  uninstall finding contract; manager-owned records can produce exact sealed
  dry-run action plans while protected/unknown/local bundles remain blocked.
- [ ] Platform-specific manager, ownership, ACL, reparse/symlink, locked-file,
  cross-filesystem, and partial-failure proof.
- [x] One narrow exact manager-owned uninstall apply boundary now reuses the
  shared identity-bound external-effect executor, destructive confirmation,
  cancellation, receipt, and fresh installed-software verification.
- [ ] Broad uninstall mutation remains blocked pending dependent/shared-
  component review, platform manager/ownership proof, rollback/manual recovery,
  and disposable-host runtime evidence.

See [`action-planning.md`](action-planning.md).

## Phase 6 — interactive UX and site

- [x] Interactive terminal flow, section navigation, details panel, command
  rail, responsive layout tiers, mouse capture, one-second monitor refresh,
  cancellable dashboard loading, refresh generation invalidation, and stale-
  result suppression.
- [x] One live installed-software section with per-item details/uninstall posture,
  bottom-safe row selection, mouse-wheel scrolling, and exact CLI action entries.
- [ ] Manual Windows Terminal/PowerShell TUI smoke and installed-binary refresh.
- [ ] Linux/macOS terminal-emulator, SSH/tmux/screen, Unicode, no-color,
  accessibility, and restoration smoke on final artifacts.
- [ ] Direct TUI confirmation/recovery flows for actions that become production-
  supported; update `U` uses the shared updater path, while uninstall and
  recovery remain CLI-only until their separate gates close.
- [x] Add parser-covered Bash/Zsh/Fish/PowerShell completion output and a
  committed `rz0(1)` manual page.
- [ ] Complete localization policy, direct TUI recovery UX, migration/repair
  guidance, and screen-reader/human review.
- [ ] Website parity and final brand-asset pass after explicit production/site
  approval; source edits may trigger the connected deployment.

## Phase 7 — platform, trust, and distribution gates

- [x] Generic process-PATH inventory compiles for supported platform families.
- [x] Public module threat model, capability, signing, isolation, transaction,
  revocation, and staged-approval design.
- [x] Bounded macOS application-bundle and Linux desktop-entry adapters.
- [x] Separate synthetic security/integrity digest classifier with report-only
  mismatch posture and no remediation claim.
- [x] Add bounded metadata-only MacPorts/Apple receipt/dpkg/pacman and
  launchd/systemd service/persistence adapters with explicit source status.
- [ ] Complete remaining in-scope RPM/DNF/Snap/AppImage and richer
  service/persistence/driver/package sources after scope freeze.
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
- [x] Expose the effective immutable foundation configuration as a path-free,
  non-authorizing `rz0 config` text/JSON review surface.
- [x] Strict privacy-safe text/JSON foundation diagnostics with exact typed checks
  and no host, user, current-directory, environment-value, or raw-path output.
- [x] Add a deterministic foundation support-report contract, a separate
  stdin/stdout report/export source module, and an integrated privacy-reviewed
  `rz0 report` foundation surface that omit raw inputs and authority; signed
  lifecycle and final-artifact platform proof remain open.
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
  primitives into a shared process-host foundation; production Windows launch
  uses an explicit inherited-handle list and pre-start Job Object assignment.
- [x] Add the bounded developer-only first-party inventory invocation lane with
  complete package revalidation, exact executable binding, cleared environment,
  short-lived challenge, bounded process host, and read-only response validation;
  production module execution, receipts, sandboxing, and third-party trust
  remain blocked.
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
  descriptor path, macOS path identity/digest revalidation, and Windows
  deny-write/delete handle lease.
- [x] Integrate Linux/Windows executable leases through guarded test-host spawn.
- [ ] Adversarially prove Linux/macOS/Windows binding in production-contained
  hosts, validate descriptor/handle inheritance and the Windows pre-start Job
  Object path on supported editions, enforce capabilities, and complete
  sandbox/isolation runtime tests.
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
  performance evidence schema for version/doctor/scan/apps/monitor/report/
  dashboard paths plus versioned PTY TUI first-frame/refresh-request timing;
  refresh-completion and target-native evidence remain open.
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
- [ ] Produce release-reviewed checksummed/provenance-bound artifacts and honest
  unsigned-platform warnings (plus signed/notarized artifacts only if the
  separately approved key/account path exists), SBOM/notices, installers,
  package channels, offline/update/rollback paths, support/incident runbooks,
  and final release evidence.
- [ ] Declare production readiness only after every required acceptance matrix
  cell passes or has an approved tested not-applicable outcome.

See [`production-readiness.md`](production-readiness.md).

## Phase 9 — documentation, operations, and public release closure

- [x] Add a documentation precedence/index map and reconcile the post-pause
  product status, command surface, write boundaries, and validation totals.
- [x] Add the explicit full-system modular end state, enable/disable semantics,
  module contract, delivery waves, and next-shift engineering handoff.
- [ ] Keep every command/help/TUI/module/site claim synchronized as behavior
  changes; add automated documentation/schema drift checks only after workflow
  approval.
- [x] Add current user, platform, troubleshooting, recovery, and privacy/sharing
  guides covering the implemented CLI/TUI and fail-closed boundaries.
- [ ] Complete migration, repair, production uninstall, administrator, and
  support-runbook documentation after those workflows and ownership are frozen.
- [ ] Complete vulnerability intake, supported-version policy, incident and
  compromised-release response, key custody/revocation, and release rollback.
- [ ] Complete beta and release-candidate plans, telemetry/crash-reporting
  decision, support ownership, compatibility-lab evidence retention, and
  go-live/rollback criteria.
- [ ] Review and explicitly approve any GitHub workflow, public release, package
  submission, website deployment, production credential, paid service, or
  recurring/quota-consuming automation before performing it.
