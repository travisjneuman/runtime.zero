# Module Trust and Execution Gate

This document defines the security work required before `rz0` may install or
execute a feature module. It is a design gate, not an implementation or maturity
claim.

Today the core validates local JSON manifests, hashes explicitly listed local
files, plans installation without writes, and inspects registry/receipt shapes.
The first-party inventory package is separately buildable; the core embeds its
library only as a bounded read adapter and does not install, activate, or execute
its lifecycle package/development binary. A developer-only staging trial now
accepts one locally selected read-only first-party package after
manifest/package-file verification and detached public test-key verification.
It copies held, verified bytes into a private runtime.zero-owned module path,
publishes a stage receipt and transaction evidence, and leaves the installed
registry unchanged. It is not a production installer and does not authorize
activation, invocation, or execution. The read-only module-status review binds
that stage receipt back to the immutable committed transaction journal head and
commit receipt, degrading the entry if evidence is missing or tampered. The
core-owned manager updater is a
separate narrow execution lane and does not authorize module execution.

This gate is the prerequisite for the end-state product described in
[`engineering-handoff.md`](engineering-handoff.md): users must be able to choose
which verified modules are enabled without allowing an untrusted package,
dependency, provider, or module-specific policy file to expand authority. The
initial seven release families and every later system-management module use this
same trust, capability, isolation, transaction, and recovery gate.

## Threat model

The future design must assume:

- a package or manifest may be malicious, malformed, replaced after validation,
  or copied through a symlink/reparse path;
- a legitimate signing key, release account, package feed, or maintainer machine
  may be compromised;
- a module may request more authority than its advertised purpose needs;
- output may contain paths, usernames, installed-app names, or other local
  evidence that becomes sensitive when shared;
- a crash or power loss may interrupt staging, registry, receipt, quarantine, or
  rollback writes;
- shell quoting, environment inheritance, current-directory search, and PATH
  resolution can turn benign command intent into arbitrary execution;
- a valid checksum proves byte equality only, not publisher identity or safety.

## Trust tiers

1. **Core foundation** — code compiled into `rz0`; normal repository review and
   release controls apply.
2. **First-party source package** — repository source such as
   `modules/inventory`; buildable by developers but not installable by the core.
3. **First-party signed artifact** — future immutable artifact with verified
   provenance, signature, digest, and release metadata.
4. **Third-party package** — blocked until signing, identity, permissions,
   isolation, revocation, reporting, and abuse-response policies are complete.

A tier is not inherited from a manifest string. Publisher identity must be tied
to verified release metadata and an approved key policy.

## Capability grant model

Before execution, every module must declare narrow capabilities that the core
can display and enforce. Initial capability families should distinguish:

- environment and filesystem metadata reads;
- bounded file-content reads under named roots;
- registry/configuration reads;
- exact executable probes;
- package-manager planning;
- network access by host/transport purpose;
- writes to runtime.zero-owned state;
- quarantine/restore;
- manager-native update or uninstall requests;
- privileged/admin operations.

Unknown capabilities fail closed. Grants should be versioned, explicit for each
invocation, and visible in text/JSON/TUI without relying on color. Read-only
modules must not receive mutation, network, credential/session, persistence, or
account capabilities.

Manifest permission schema 1 now implements the read-only subset for process
environment, filesystem metadata, persisted environment registry, application
registry, bounded application filesystem reads, and exact command probes. It
requires application inventory and command-probe permissions to be explicit. This validates declarations only; it is not yet a
core runtime grant or execution mechanism. Mutating/network capabilities remain
outside schema 1.

## Package verification sequence

A future installer must verify in this order:

1. Resolve a local immutable input selected by the user; never execute while
   downloading.
2. Reject absolute/traversal/URL-like/backslash-ambiguous package paths,
   symlinks, reparse points, unsupported file types, oversized files, duplicate
   paths, and undeclared files according to the approved package format. The
   optional `integrity.complete_file_set` review mode now implements bounded
   recursive undeclared-file rejection for local package review.
3. Verify package digest and every declared file digest.
4. Verify a supported detached signature against an approved key and package
   identity/version binding.
5. Verify provenance/release metadata and freshness/revocation policy.
6. Re-open or stage verified bytes without trusting mutable source paths; defend
   against validation-to-use replacement. `crates/artifact-identity/` now
   provides the bounded same-open-handle identity/digest primitive and Unix
   held-root no-follow component traversal. Compile-checked Windows NT root-
   relative state operations now exist separately, but package traversal,
   owner/DACL runtime proof, and platform execution binding remain gated.
7. Display capabilities, destination, write set, risk, receipt, rollback, and
   quarantine plan.
8. Require explicit confirmation for any write or elevated capability.

`crates/module-trust/` now implements local detached Ed25519 verification with
RFC-derived public test fixtures. Its canonical message binds scheme, key ID,
package ID/version, and exact manifest SHA-256. The caller must select a matching
non-revoked test key that explicitly authorizes the package ID; the envelope
cannot self-authorize a key. See
[`signature-verification.md`](signature-verification.md).

This settles only the bounded test-key scheme. Production key custody, release
authorization, rotation, compromise response, recovery, provenance,
transparency/freshness, and reproducible public verification instructions remain
undecided and required before release use.

## Execution isolation

The initial executable-module design should prefer a separate process and a
versioned stdin/stdout JSON protocol over in-process dynamic libraries.
`crates/module-protocol/` validates a fixture-only invocation preview and
`not_executed` response: exact receipt-relative executable/digest metadata,
cleared name-allowlisted environment, least-privilege read grants, mandatory
redaction, and bounded time/I/O. Module authorization/attempt remains false.
Under the non-default `protocol-test-child` feature, integration-test support
copies and executes only a Cargo-built helper in a marked OS-temp root to test
framing, exact environment names, concurrent output drains, bounded retention,
and timeout teardown. Native Unix tests now reject an observed inheritable
non-standard descriptor before spawn and terminate a helper-spawned descendant
through a fresh process group. The core and inventory module do not use this
lane.

A future host must:

- invoke an exact receipt-recorded executable path without a shell or PATH
  search;
- set an explicit working directory and minimal environment;
- close or explicitly grant inherited handles;
- bound runtime, output, request size, and concurrent work;
- validate every request against the invocation's capability grant;
- treat stdout/stderr as untrusted bounded data;
- kill timed-out children and report partial evidence;
- keep credentials, browser sessions, project contents, and unknown user data
  outside default grants;
- evaluate platform sandbox primitives separately for Windows, macOS, and Linux
  rather than claiming portable isolation prematurely.

A process boundary alone is not a sandbox. Module execution remains blocked
until capability enforcement and platform isolation are tested. The schema-1
production execution assessment makes that block machine-checkable across the
canonical artifact/capability/identity/process/runtime/transaction gate set; it
has no authorization decision. See
[`production-readiness.md`](production-readiness.md).

The first explicit developer-trial process lane now consumes this foundation
for one promoted `first-party.inventory` package. It requires a complete
immutable package file set and a `receipts/install-*.json` test-key trial
receipt, binds the declared Rust executable through the host, and accepts only
the path-redacted read-only inventory response. It does not activate state,
persist a lifecycle receipt, execute third-party code, or change the blocked
production decision. The host's bounded transport and identity binding are
evidence, not sandbox or production authorization.

## Transaction, receipt, and rollback rules

Future writes must use a transaction with an immutable intent record:

- stage under a runtime.zero-owned location;
- record package identity, verified digests, source/provenance, capability grant,
  exact write set, prior state, and rollback steps;
- write files durably before atomically publishing registry state;
- never claim installation if registry/receipt publication is incomplete;
- quarantine replaced runtime.zero-owned files instead of deleting them;
- rollback only receipt-listed runtime.zero-owned paths;
- stop for manual review on mismatched, missing, shared, credential/session,
  project, backup, or unknown paths;
- make repair/migration separate explicit commands, never implicit startup work.

The existing schema-1 receipts are validation fixtures, not sufficient proof of
transactional installation.

## Distribution and revocation

Remote distribution requires a separately approved design for:

- immutable release artifacts and checksums;
- signing key storage/rotation/recovery;
- provenance attestations;
- feed metadata and rollback/freeze protection;
- revocation and compromised-release response;
- mirrors/CDNs and offline behavior;
- telemetry/privacy posture;
- release reproducibility, dependency/license review, and vulnerability audits.

No direct-run internet bootstrap, package feed, automatic update, release
workflow, or third-party submission path should appear before these controls
exist and are manually exercised.

## Approval gates

Implementation may proceed only in bounded stages:

1. **Implemented:** permission/capability schema and fixture validation.
2. **Implemented:** local detached Ed25519 verification with public test keys
   only; no signer, private key, production trust root, or installer integration.
3. **Implemented as tests only:** immutable staging-plan validation and atomic
   publication simulation under a marked direct child of the OS temp root.
4. **Implemented as a gated foundation API:** receipt-bound quarantine/restore
   execution with exact private roots, no-replace moves, record verification,
   journal/receipt publication, and tamper/conflict rejection. Its tests use
   disposable roots; domain invocation and cross-platform recovery remain
   blocked.
5. **Implemented as a native test-helper slice:** fixture-only first-party
   invocation/not-executed module contract plus an explicit-feature Cargo helper
   transport. The helper slice proves bounded JSON framing, exact cleared
   environment names, an explicit working directory, concurrent output drains,
   fail-closed output ceilings, Unix inheritable-descriptor refusal, and Unix
   process-group timeout teardown including a sleeping descendant. It does not
   execute a module or provide a core API. Same-open-handle identity and
   non-authorizing Linux/Windows spawn leases are implemented for guarded test
   builds; macOS binding, descriptor/handle-audit races, Windows production Job
   control, core module-host integration, and platform sandbox/capability
   isolation remain open. A separate schema-1 production assessment records the
   complete gate set but cannot authorize execution.
6. **Implemented as a developer-only signed artifact trial:** the explicit
   `modules install --developer-trial` path stages one local read-only
   first-party package after test-key verification, held source identity
   revalidation, private store checks, exact confirmation, transaction/receipt
   publication, and post-copy byte verification. It leaves the installed
   registry unchanged and grants no activation or execution authority. The
   signed test key remains a fixture trust root only.
7. Separately approved release/distribution work.
8. Third-party threat model and governance last.

See [`module-process-protocol.md`](module-process-protocol.md) for the schema-1
preview and its no-execution response boundary.

The stage-3/4 filesystem writes and stage-5 helper launch still exist only in
integration-test support, require marked/prefixed direct OS-temp children, and
are removed by test cleanup. The test-child model is compiled only under an
explicit feature. The developer trial is the first bounded core write path for
module bytes, but it is deliberately not installed-registry publication,
activation, module execution, production trust, or public distribution. The
separately reviewed leftovers exact-file lane is not module execution or
staging: it only moves one runtime.zero-owned module-store file through the
receipt-bound foundation quarantine executor after exact confirmation. Each
stage must preserve a no-execution product mode and stop before the next gate.
Destructive cleanup, credential/session handling, persistence, account actions,
production deployment, and recurring automation remain outside this design
without explicit approval.
