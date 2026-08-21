# Release Acceptance Contract

`crates/release-contract/` converts the production matrix into a finite,
machine-validated target × module × lifecycle assessment. It tracks evidence but
cannot authorize a release.

## Scope targets

A release target binds a normalized ID to:

- Windows client/server, macOS, Ubuntu, Debian, RHEL, or Arch;
- exact generation and real edition/variant;
- x86, x86-64, or ARM64 architecture;
- portable ZIP, DMG, PKG, DEB, RPM, or Arch package artifact;
- release-blocking, legacy-compatibility, or research tier;
- current vendor-support truth.

Vendor-retired systems cannot be labeled release-blocking supported systems.
They remain mandatory compatibility targets where the approved matrix requires
them.

## Exact acceptance cross-product

Every sorted target expands to exactly 84 cells: seven frozen module families ×
12 lifecycle stages. The validator requires the full canonical Cartesian product
with deterministic IDs and ordering. Up to 256 targets and 21,504 cells are
allowed; generation cannot allocate an unbounded matrix.

The module families are inventory/environment, updater, uninstall, leftovers,
cache management, security/integrity, and report/export. Lifecycle stages bind
requirements/threat/privacy, adversarial fixtures, discovery/normalization,
finding/action evidence, text/JSON/TUI, capability/isolation, confirmation,
transaction/recovery, post-action verification, lifecycle/repair/migration,
test/performance/soak, and final security/accessibility/release review.

## Evidence states

- `missing` cannot carry mechanism, evidence, or rationale;
- `proven` requires a bounded mechanism and evidence reference;
- `not_applicable` requires an evidence-backed rationale and cannot claim a
  mechanism.

Targets and cells are bounded, unique, strict, and reject unknown fields. Schema
1 has only `decision: blocked` and requires `release_authorized: false`. Even a
synthetic all-proven matrix cannot authorize publication. A later authorization
contract requires independent review of the complete frozen RC scope and all
external release capabilities.

This crate does no host discovery, artifact execution, network access,
publication, signing, or mutation. It is the canonical completeness ledger shape,
not completion evidence itself.

The scriptable inspection surface is `rz0 release status --assessment
<assessment.json> [--format text|json]`. It reads one explicit bounded regular
file, validates the complete target × module × lifecycle cross-product, and
reports the blocked schema-1 decision. It does not discover targets, authorize
publication, sign artifacts, or change the assessment.

See [`production-readiness.md`](production-readiness.md),
[`support-policy.md`](support-policy.md), and
[`windows-compatibility.md`](windows-compatibility.md).
