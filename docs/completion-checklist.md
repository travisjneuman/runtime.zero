# Product Completion Checklist

This is the consolidated remaining-work checklist for the frozen
`runtime.zero 1.0` product described in
[`production-readiness.md`](production-readiness.md). It is exhaustive at the
workstream level as of 2026-08-09. The machine-generated release ledger remains
the exhaustive cell-level authority: every frozen target × seven module families
× 12 lifecycle stages must be `proven` or evidence-backed `not_applicable`.

The seven families are the initial release gate, not the full product horizon.
The end state is documented in [`engineering-handoff.md`](engineering-handoff.md):
every additional system-management capability or provider is an independently
versioned module that users can enable or disable. This checklist must be
extended with new platform/provider/lifecycle cells whenever a future module is
formally admitted to a release scope.

A future feature, commercial module, or third-party ecosystem is not part of
1.0 unless it is explicitly added to the frozen scope. Adding one also adds its
full platform/lifecycle acceptance cells.

## 1. Freeze the release scope

- [ ] Freeze the exact 1.0 user journeys, feature requirements, non-goals, and
  measurable acceptance criteria.
- [ ] Freeze the release-candidate OS generations, real editions/variants,
  architectures, filesystems, privilege modes, terminals, shells, managers, and
  artifact/install channels.
- [ ] Verify current vendor lifecycle/support truth at RC freeze and separate
  supported targets from legacy compatibility and research.
- [ ] Decide which technically impossible or unavailable legacy cells are
  evidence-backed `not_applicable` rather than silently omitted.
- [ ] Freeze schema/API/CLI/JSON compatibility, migration, deprecation, and
  minimum-Rust/toolchain policies.
- [ ] Freeze command-specific latency, memory, disk, output, network, retention,
  and concurrency budgets from measured final artifacts.
- [ ] Generate the exact release ledger and assign stable acceptance IDs to
  every target × module × lifecycle cell.

## 2. Harden current core write paths

- [x] Bind every supported updater manager executable from a verified opened
  artifact or equally reviewed platform identity primitive through the actual
  spawn on macOS/Linux; Windows uses the pre-start Job Object/handle-list host
  but remains gated on runtime identity and platform proof.
- [ ] Replace path allowlisting alone with adversarially proven replacement-race
  closure on Linux/macOS/Windows and complete platform containment proof.
- [x] Integrate updater confirmation, journal, receipt, and final state through
  the canonical transaction/commit-receipt/recovery model rather than a
  parallel partial flow.
- [x] Record exact manager write intent and verified outcome in the durable
  transaction chain.
- [x] Make updater receipt publication interruption-safe after a successful
  manager command and provide deterministic reconciliation for missing,
  duplicate, partial, or conflicting evidence.
- [x] Add a fresh receipt-bound recovery challenge and idempotent local journal
  completion path that cannot rerun a manager or authorize automatic mutation.
- [ ] Propagate cancellation through live discovery, process polling/teardown,
  journal, receipt, verification, and every write boundary.
- [ ] Implement manager-native rollback where supported and an explicit tested
  manual-recovery path where it is not.
- [ ] Prove drift, stale challenge, replay, timeout, nonzero exit, truncated
  output, verification mismatch, low space, permission failure, process crash,
  host reboot, and power-loss outcomes on disposable hosts.
- [ ] Prove `store init --yes` owner/permission/ACL, idempotency, partial-state,
  rollback guidance, and recovery on every supported platform/filesystem.
- [ ] Keep update and store writes unavailable on cells lacking this proof.

## 3. Complete process, capability, privilege, and network enforcement

- [ ] Close Unix inheritable-descriptor audit races and define an explicit
  descriptor allowlist at spawn.
- [ ] Implement race-free Windows suspended creation, inherited-handle
  allowlisting, Job Object assignment, process-tree teardown, and reap proof.
- [ ] Prove Linux/macOS descendant containment and document/reject session or
  namespace escape cases.
- [ ] Implement the production module/manager process host with exact executable,
  working directory, environment, stdin/stdout/stderr framing, timeout,
  concurrency, cancellation, and partial-evidence semantics.
- [ ] Enforce every granted filesystem, registry, process, network, manager,
  state-write, quarantine, restore, and elevation capability at the OS boundary.
- [ ] Prove ungranted capabilities are denied on Windows, macOS, and Linux.
- [ ] Implement and test Windows isolation primitives, macOS sandbox/code-
  signing constraints, and Linux namespace/seccomp/landlock policy without
  claiming a false portable sandbox.
- [ ] Define explicit offline/default-deny network enforcement and approved
  destination/purpose rules for availability and write operations.
- [ ] Define least-privilege elevation brokers or explicit unsupported outcomes;
  never fall back to hidden `sudo`, UAC, or interactive helper behavior.
- [ ] Enforce process, CPU, memory, I/O, output, child-count, and wall-time
  budgets under normal, hostile, and resource-pressure conditions.

## 4. Complete secure filesystem, transaction, quarantine, and recovery

- [ ] Prove Windows private initial ACL creation, exact owner/DACL inspection,
  inherited ACL handling, reparse/File-ID policy, atomic rename, and directory
  flush on every supported client/server filesystem.
- [ ] Prove Unix owner/mode/no-follow behavior across supported macOS/Linux
  filesystems, case modes, mount options, and privilege contexts.
- [ ] Test symlink, reparse point, hardlink, bind mount, root replacement,
  locked file, long path, Unicode, case collision, sparse file, low-space,
  read-only, network/removable, and cross-filesystem cases.
- [ ] Complete production write-ahead journals, immutable heads, receipts,
  registry-last publication, idempotency, conflict handling, and bounded
  retention for every mutation class.
- [ ] Implement exact rollback execution from verified prior evidence, including
  interrupted rollback continuation and mismatch refusal.
- [x] Promote quarantine/restore from test helpers to receipt-scoped foundation
  APIs with exact roots, no-replace moves, record verification, journal
  snapshots, and filesystem-effect receipts.
- [x] Observe cancellation through the receipt-bound quarantine/restore
  transaction boundaries and classify post-move cancellation as recovery
  required.
- [x] Add an exact CLI restore lane that derives a fresh restore plan from one
  validated quarantine record and reuses the receipt-bound executor; occupied,
  symlinked, drifted, and unsupported destinations fail closed.
- [ ] Bind the executor to each domain's ownership/provenance findings and
  expose the reviewed action through the shared CLI/TUI confirmation workflow;
  complete retention, metadata fidelity, capacity, and cross-filesystem policy.
- [ ] Keep permanent deletion a separate exact-confirmation action with no
  implicit retention expiry.
- [ ] Prove abrupt process termination, kernel/host reboot, power loss, disk
  corruption, partial sync, competing writer, tamper, and stale-lock recovery.
- [ ] Provide CLI/JSON/TUI recovery status and operator guidance for every
  durable partial state.
- [x] Add bounded read-only CLI/JSON recovery journal inspection and a TUI
  summary of checked/invalid/action-required journals without creating writer
  locks or adding rollback authority; this is evidence for the broader gate,
  not proof of every platform or power-loss state.
- [ ] Ensure credentials, sessions, browser profiles, projects, backups, user
  content, shared components, and unknown data remain blocked unless a future
  separately scoped workflow explicitly changes the 1.0 policy.

## 5. Complete module package trust and lifecycle

- [ ] Define production package format, complete-file manifest, immutable input,
  undeclared-file policy, and package-size/resource limits.
- [ ] Define production signing keys, release authorization, custody, offline or
  hardware storage, rotation, recovery, threshold policy, and compromise
  response.
- [ ] Implement production signature/provenance/freshness/transparency/revocation
  verification and rollback/freeze protection.
- [ ] Bind package, manifest, every file, publisher, version, source release,
  capability declaration, and receipt into one reviewed trust decision.
- [ ] Implement production immutable staging without reopening mutable source
  paths after verification.
- [ ] Implement foundation-owned install, activate, invoke, deactivate, repair,
  migrate, upgrade, and uninstall execution.
- [ ] Prove every lifecycle transition, interruption, rollback, compatibility,
  downgrade, schema migration, and stale-version behavior.
- [ ] Connect installed state, receipts, module directories, process protocol,
  capability grants, transactions, diagnostics, and recovery without module-
  private alternatives.
- [x] Add a path-redacted, read-only `modules status` surface that composes
  registry/receipt evidence and reports `installed_inactive` or `degraded`
  without claiming activation or execution authority.
- [ ] Build and validate signed first-party artifacts for all seven module
  families and supported targets.
- [ ] Keep third-party packages blocked until publisher governance, review,
  permission UX, sandboxing, revocation, abuse response, and support ownership
  are separately complete.

## 6. Complete inventory and software identity

- [x] Define durable multi-source software identity/provenance beyond normalized
  display-name heuristics while preserving every source and disagreement.
- [ ] Add adversarial reconciliation fixtures for app/cask duplicates, aliases,
  renamed/moved apps, multiple versions, conflicting managers, missing metadata,
  split packages, shared components, and unknown ownership.
- [ ] Complete Windows installed applications, MSI/MSIX/AppX, Winget,
  Chocolatey, Scoop, services, persistence, PATH/environment, drivers, and
  relevant package evidence within the frozen scope.
- [ ] Complete macOS app bundles, Homebrew, package receipts, MacPorts,
  launchd/services, persistence, and relevant package evidence.
- [ ] Complete Linux APT/dpkg, DNF/RPM, pacman, Snap, Flatpak, AppImage,
  services, XDG, persistence, and frozen distro-specific sources.
- [ ] Decide and implement Nix, language managers, containers, browser
  extensions, and other discovered sources as in-scope or explicit exclusions.
- [ ] Establish trusted publisher/product IDs and active-version/linkage evidence
  where each platform exposes it.
- [x] Preserve useful partial source results, deterministic ordering, redaction,
  bounded scans, and explicit unsupported/unavailable states.
- [ ] Add real runtime privacy/redaction/source-agreement tests on every target.
- [ ] Add explicit `apps` and interactive TUI startup/refresh operations to a
  versioned performance contract.

## 7. Complete updater on every platform

- [ ] Freeze supported update managers and exact installed-only behavior per
  platform/version.
- [ ] Prefer stable machine interfaces; prove every unavoidable text parser
  against version, locale, encoding, warning, partial-output, and source-
  agreement fixtures.
- [ ] Finish safe Winget, Zypper, Snap, and Flatpak parsing or mark exact cells
  unsupported; expand Homebrew/APT/DNF/Pacman/MacPorts proof.
- [ ] Prove manager executable discovery, version compatibility, offline mode,
  source agreements, network behavior, lock contention, privilege, and
  no-surprise-install semantics.
- [ ] Prevent package-manager self-update or metadata mutation during a declared
  read-only availability check, or disclose and separately authorize it.
- [ ] Pin exact package/source/target versions and invalidate plans on any
  installed or remote evidence drift.
- [ ] Implement serial one-item execution, pause/resume, cancellation, rollback,
  verification, and recovery for every supported manager.
- [x] Add direct TUI review/confirmation through the same CLI safety gates; do
  not create a second authority path.
- [ ] Prove real updates only on disposable hosts with synthetic/noncritical
  packages before any support claim.
- [ ] Complete updater CLI/JSON/TUI, docs, accessibility, performance, security,
  privacy, support, and all release-ledger cells.

## 8. Complete uninstall, leftovers, cache, integrity, and report/export

### Uninstall

- [x] Convert live catalog evidence into shared uninstall findings and exact
  action plans rather than extending the temporary `uninstall_review` model.
- [ ] Implement manager-native uninstall adapters, dependent/shared-component
  review, elevation, restart/reboot, verification, rollback, and recovery for
  every frozen manager/platform.
- [ ] Implement race-resistant root-relative macOS bundle quarantine/restore and
  equivalent non-manager platform methods where explicitly in scope.
- [ ] Keep protected system and unknown/shared data blocked and prohibit direct
  recursive deletion shortcuts.

### Leftovers

- [x] Add a bounded, path-free, read-only runtime.zero-owned module/log evidence
  adapter and expose its shared finding contract through the CLI and TUI.
- [x] Add bounded, path-free, read-only evidence for unreferenced runtime-owned
  receipt files when the installed-module registry is valid.
- [x] Add one exact module-store file dry-run quarantine plan with re-read
  digest/size evidence, logical paths, and no absolute source-root disclosure.
- [x] Bind that exact plan to a short-lived confirmation challenge and the
  receipt-bound foundation quarantine executor; challenge-only mode remains
  write-free and the apply lane moves only one revalidated file.
- [ ] Implement the remaining bounded post-uninstall ownership adapters for shims,
  launch/service entries, exact runtime-owned files, and approved roots.
- [ ] Prove stale versus user-valued/shared/config/log/backup evidence and keep
  ambiguous data report-only.
- [ ] Implement low-risk quarantine/restore with retention, rollback, and fresh
  verification on every supported platform.

### Cache management

- [x] Add a bounded, path-free, read-only known-root evidence adapter and expose
  its shared finding contract through the CLI and TUI.
- [x] Add one exact runtime.zero cache-file plan and confirmation-bound
  foundation quarantine lane; manager, user, shared, and unknown cache roots
  remain report-only.
- [x] Define manager/runtime cache ownership, a 30-day review-age threshold, a
  16 MiB runtime review-size budget, bounded lock-marker active-use signals,
  and explicit exclusions; absence of a marker is not proof of inactivity.
- [ ] Implement bounded live discovery and quarantine/restore without broad
  home/profile/drive scans.
- [ ] Prove low-space, locked, concurrent-writer, permission, and restore
  behavior; keep user/shared/unknown caches report-only.

### Security and integrity

- [x] Expose strict fixture and bounded exact-file digest evidence through the
  shared report contract while refusing remediation and trusted-baseline claims.
- [ ] Define exact checks and non-goals without malware-removal or unsupported
  assurance claims.
- [ ] Establish trusted, versioned baselines and provenance for every integrity
  comparison.
- [x] Implement one bounded live exact-file read with path-safe opened-artifact
  identity and path-free output; mismatch severity, incident guidance,
  privacy handling, and false-positive review on all platforms.
- [ ] Keep remediation out of 1.0 unless separately scoped through the complete
  action/transaction/rollback matrix.

### Report and export

- [x] Integrate the strict summary exporter through signed module lifecycle or
  an explicitly chosen foundation surface.
- [ ] Define optional support-bundle attachments, user review, local output,
  encryption, retention, size, and external-sharing workflow.
- [x] Preserve omission of raw paths, identities, app names, process output,
  credentials, and free-form sensitive warnings by default.
- [ ] Prove deterministic text/JSON, accessibility, platform parity, support
  usability, and final-artifact behavior.

## 9. Complete CLI, JSON, TUI, accessibility, and documentation

- [ ] Freeze stable command names, options, exit codes, JSON schemas, error
  codes, additive compatibility rules, and deprecation policy.
- [ ] Complete action, confirmation, progress, cancellation, recovery,
  rollback, unsupported, and partial-evidence UX in CLI and TUI.
- [ ] Add shell completions, manual pages, examples, troubleshooting, migration,
  repair, rollback, uninstall, privacy/sharing, and support guides.
- [ ] Decide localization policy and prove locale-independent machine output and
  safe Unicode/control-character behavior.
- [ ] Complete keyboard-only, mouse, no-color, high-contrast, reduced-motion,
  small-terminal, resize, alternate-screen, crash-restoration, SSH, tmux, and
  screen behavior.
- [ ] Complete human screen-reader/accessibility review; do not infer it from
  text labels alone.
- [ ] Test PowerShell and `cmd.exe` quoting/pipes/redirects/exit codes plus
  representative macOS/Linux shells and terminals.
- [ ] Keep website, README, help, TUI labels, module docs, safety policy, and
  shipped behavior synchronized.
- [ ] Update the public website mock to the real five-workspace TUI and current
  capability boundaries after explicit deployment approval.
- [ ] Select/finalize brand assets, favicon, README/social art, screenshots, and
  contrast evidence without promoting candidate assets silently.

## 10. Complete platform and compatibility matrices

- [ ] Link/package and run final modern Windows x86-64/ARM64/x86 artifacts on
  every real in-scope client/server edition and variant.
- [ ] Produce and run the Windows-7-baseline compatibility artifacts; record
  transparent impossible outcomes for Server 2008/Itanium or other infeasible
  cells.
- [ ] Complete Windows registry, ACL, reparse, locked-file, long-path, console,
  Terminal, PowerShell, `cmd.exe`, manager, installer, elevation, process, and
  recovery matrices.
- [ ] Run final Apple Silicon and Intel artifacts on every frozen macOS release/
  hardware pair; Rosetta is not Intel-hardware proof.
- [ ] Complete macOS filesystems, Gatekeeper/unsigned warnings, terminals,
  managers, launchd, privilege, sandbox, packaging, and recovery matrices.
- [ ] Run final x86-64/ARM64 artifacts on every frozen Ubuntu, Debian, RHEL, and
  Arch generation/snapshot.
- [ ] Complete Linux filesystems, containers/restricted `/proc`, terminals,
  managers, systemd/services, sandbox, privilege, packages, and recovery
  matrices.
- [ ] Test normal and adverse filesystem states: case sensitivity, ACLs,
  symlinks/reparse, locks, low space, read-only, cross-filesystem, removable,
  and network mounts where in scope.
- [ ] Use artifact-only clean compatibility hosts and snapshot-backed disposable
  mutation hosts; never count build runners as runtime proof.
- [ ] Record exact host image, artifact digest, harness version, result, and
  evidence retention for every acceptance cell without private host data in
  public output.

## 11. Complete quality and security assurance

- [ ] Expand unit/integration/end-to-end coverage for every module/platform/
  lifecycle state and user journey.
- [ ] Add property and fuzz testing for every untrusted schema, parser, path,
  digest, journal, receipt, protocol, and manager-output boundary.
- [ ] Add concurrency/race testing for files, processes, registry/state,
  cancellation, refresh, manager locks, and competing runtime.zero instances.
- [ ] Add deterministic and real fault injection for every I/O/process/network/
  transaction boundary.
- [ ] Add performance profiling, memory/disk/network budgets, regression
  thresholds, sustained-load and soak evidence on final artifacts.
- [ ] Audit every unsafe Rust/FFI block and platform structure/layout against
  authoritative APIs; add sanitizers/static analysis where supported.
- [ ] Complete threat models and privacy reviews per module/platform, including
  terminal injection and local inventory sensitivity.
- [ ] Run fresh RustSec/dependency/license/source audits and resolve or document
  duplicates, advisories, yanked packages, unsupported dependencies, and notice
  obligations.
- [ ] Obtain an independent security review/penetration assessment appropriate
  to the write/trust surface.
- [ ] Complete secret, private-path, binary-content, generated-artifact, and
  public-claim scans for the exact RC.

## 12. Complete packaging, installation, update, and distribution

- [ ] Rebuild reproducible final portable artifacts for every frozen target and
  bind version, source commit, binary, SBOM, notices, manifest, and checksum.
- [ ] Independently reproduce artifacts where toolchain/platform permits and
  document honest container-format variance.
- [ ] Complete target-specific legal/license/source-offer review.
- [ ] Build and test Windows portable ZIP and any approved installer across
  install, upgrade, repair, rollback, uninstall, PATH, privilege, and warning
  behavior.
- [ ] Build and test macOS archives/DMG and any approved PKG across install,
  upgrade, rollback, uninstall, Gatekeeper, quarantine attributes, and both
  architectures.
- [ ] Build and test Linux archives, DEB, RPM, and Arch packages across native
  manager install, upgrade, downgrade, rollback, uninstall, offline, and
  dependency behavior.
- [ ] Define safe self-update and module-update channels, metadata, freshness,
  rollback/freeze protection, mirrors, offline verification, and compromised-
  release response.
- [ ] Provide public checksum/provenance/signature verification instructions
  that do not overstate unsigned, ad-hoc, test-key, or keyless evidence.
- [ ] Complete clean-host install and first-run tests using only user-facing
  artifacts and standard OS facilities.
- [ ] Publish nothing until the exact RC evidence and external-write approvals
  pass.

## 13. Complete operations, support, and release governance

- [ ] Define supported-version and patch/backport policy.
- [ ] Enable and test private vulnerability intake, triage, coordinated
  disclosure, advisories, and incident response.
- [ ] Define release roles, approvals, key/credential custody, two-person review
  where appropriate, and emergency revocation/rollback.
- [ ] Decide telemetry and crash-reporting posture; remain no-network/no-
  telemetry by default unless a separately reviewed opt-in design is approved.
- [ ] Create support diagnostics, privacy-safe evidence collection, support
  ownership, escalation, retention, and user communication runbooks.
- [ ] Establish compatibility-lab image/media legality, isolation, snapshots,
  evidence retention, and reproducibility.
- [ ] Complete alpha/beta/RC entry/exit criteria, rollout, rollback, known-
  limitations review, and go/no-go process.
- [ ] Add only explicitly approved CI, release, deployment, scheduled, or quota-
  consuming automation with least-privilege credentials and rollback.
- [ ] Verify documentation, site, repository metadata, release notes, security
  policy, support policy, and public claims against exact shipped bytes.
- [ ] Run the final cross-platform safety, security, privacy, accessibility,
  compatibility, performance, recovery, supply-chain, legal, documentation,
  artifact, and remote-HEAD audit.

## 14. Final completion decision

- [ ] Every frozen acceptance ID is `proven` or has reviewed evidence-backed
  `not_applicable` status.
- [ ] Every production execution assessment is satisfied by a separately
  reviewed authorization schema; schema 1 remains non-authorizing.
- [ ] Every required module lifecycle is implemented and tested on every
  supported platform.
- [ ] Every release artifact and install/update/rollback/uninstall path passes on
  clean final-artifact hosts.
- [ ] Security, privacy, accessibility, legal, support, incident, and recovery
  reviews approve the exact RC.
- [ ] Public claims exactly match shipped behavior and unsupported cells are
  visible.
- [ ] Travis explicitly approves the external publication/deployment/package-
  channel actions for the exact RC.
- [ ] Only then declare `runtime.zero 1.0` production-complete.
