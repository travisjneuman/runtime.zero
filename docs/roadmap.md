# Roadmap

The roadmap is ordered by safety and contract dependency. A checked item means
the documented source/contract exists; it does not imply a release, installation
path, production support, or permission to cross a later gate.

## Phase 1 — foundation baseline (complete)

- [x] Public Rust CLI, safety/security/contribution docs, brand, and static site.
- [x] Read-only Ratatui/Crossterm dashboard with text/JSON fallbacks.
- [x] Module manifest validation and local SHA-256 package integrity.
- [x] Dry-run install planning and user-local store/registry/receipt contracts.
- [x] Explicit store scaffolding as the only foundation write surface.

## Phase 2 — inventory contracts and primitives (implemented; runtime proof remains)

- [x] Versioned inventory JSON and privacy/no-write fields.
- [x] Valid, duplicate, missing, malformed, invalid-entry, and
  unsupported-platform fixtures.
- [x] Bounded process-PATH normalization on Windows/macOS/Linux.
- [x] Read-only Windows User/Machine PATH registry adapters.
- [x] Allowlisted known-executable discovery without recursive scans.
- [x] Opt-in exact-path version probes with no shell, bounded output, and timeout.
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
- [ ] Installed-only availability adapters after explicit network review.
- [ ] Execution remains blocked until trust, transaction, receipt, rollback, and
  confirmation gates are complete.

## Phase 5 — uninstall, leftovers, quarantine, and restore

- [x] Manager-native priority, risk categories, blocked data classes, and
  quarantine/restore design contract.
- [x] Fixture-only uninstall/quarantine/blocked-data plan schemas and policy tests.
- [x] Test-only temporary-root transaction/quarantine/restore simulation with
  verified-copy-before-remove, conflict refusal, failure injection, and receipts.
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
- [ ] macOS/Linux package-manager, service, and persistence inventory adapters.
- [ ] Real Linux application runtime and Windows/full cross-platform compatibility matrix.
- [ ] Artifact-level license/notice and reproducibility audit; source dependency
  vulnerability/license metadata audit is recorded.
- [x] Read-only manifest permission schema with default-versus-explicit grants.
- [x] Test-key-only detached Ed25519 signature contract and strict verification.
- [x] Fixture-only immutable staging plan plus atomic temporary-root publication
  simulation bound to successful test-key verification.
- [x] Fixture-only first-party inventory invocation/not-executed response
  protocol with exact receipt path, least-privilege grants, cleared environment,
  and timeout/I/O ceilings.
- [ ] Mutating/network capability schema, child-process transport, and platform
  isolation tests.
- [ ] Signing keys, release artifacts, package publishing, bootstrap, remote
  feeds, third-party modules, deployment automation, and production actions only
  after their separate explicit approvals.

See [`module-trust-and-execution.md`](module-trust-and-execution.md).
