# Security Policy

## Supported versions

`runtime.zero` is pre-alpha. No version is production-supported and no release
currently receives a security-maintenance guarantee.

| Version | Security support |
| --- | --- |
| Repository `main` / local development builds | Best-effort development review only |
| `0.1.0` local artifacts | Unsupported pre-alpha evidence |
| Published production release | None exists |

Compatibility experiments on vendor-retired operating systems are not security
support claims.

## Reporting a vulnerability

Use a private GitHub Security Advisory for
[`travisjneuman/runtime.zero`](https://github.com/travisjneuman/runtime.zero/security/advisories/new)
when that intake is available. If it is unavailable, contact the maintainer
through the GitHub profile rather than opening a public issue with exploit or
sensitive details.

Include, when safe:

- affected source commit and platform;
- the command or contract involved;
- expected versus observed behavior;
- a minimal synthetic reproducer;
- whether any write, privilege boundary, network access, terminal escape,
  privacy leak, or recovery state was involved;
- suggested embargo or coordination needs.

Do not include real tokens, cookies, OAuth sessions, private keys, credentials,
customer/employer data, personal inventory, private paths, or unredacted host
reports. Replace them with synthetic evidence and state what was redacted.

No response-time SLA exists before the first supported release, but reports will
be triaged as maintainer capacity permits. Publication timing and attribution
should be coordinated before disclosure.

## High-priority vulnerability classes

Please report issues involving:

- command or argument injection, shell/PATH fallback, or manager substitution;
- executable replacement between validation and spawn;
- symlink, reparse-point, hardlink, traversal, root-redirection, or unsafe path
  handling;
- confirmation replay, plan/write-set drift, stale evidence, or action-scope
  confusion;
- transaction, receipt, registry, quarantine, rollback, or recovery corruption;
- escaping process-tree/resource/network/capability boundaries;
- terminal escape/control-sequence injection or failed terminal restoration;
- unbounded input, output, recursion, concurrency, retention, or resource use;
- secrets, identities, raw paths, process output, software/service inventory,
  or environment values appearing in public/share-oriented output;
- signature, digest, provenance, revocation, package identity, or trust bypass;
- protected credentials/sessions/workspaces/backups/unknown data becoming an
  actionable finding;
- updater behavior that executes an unplanned action, silently broadens network
  or elevation scope, retries automatically, or claims success without fresh
  verification.

## Current security boundary

The project intentionally separates evidence from authority:

- inventory, diagnostics, monitor snapshots, findings, dry-run plans,
  signatures, executable leases, confirmations, receipts, and release ledgers
  do not independently authorize a mutation or release;
- optional module execution remains blocked until production trust,
  provenance/revocation, exact executable identity, capability enforcement,
  platform isolation, lifecycle, transaction, rollback, and runtime-evidence
  gates pass;
- uninstall, cleanup, quarantine/restore, and module installation/activation are
  not product execution paths;
- `store init --yes` writes only runtime.zero-owned user-local scaffolding on
  platforms whose filesystem policy is enabled;
- `updates --apply` is a narrow pre-alpha core exception with fresh discovery,
  explicit network intent, plan-sealed executable identity, exact confirmation,
  exact journal events, canonical external-effect receipt evidence, bounded
  cancellable manager execution, fresh verification, and read-only recovery
  status.

The updater exception is not production-hardened. Linux direct native ELF
managers now use and revalidate a held-descriptor spawn binding. macOS exact
spawn and Windows race-free process-tree containment remain blocked. OS-enforced
network/capability isolation, full boundary cancellation, native rollback,
exact approved recovery completion, and the disposable-host fault/power-loss
matrix are still absent. See
[`docs/project-status-and-resumption.md`](docs/project-status-and-resumption.md)
and [`SAFETY.md`](SAFETY.md) before evaluating it.

## Project prohibitions

This project will not intentionally implement malware behavior, stealth
persistence, credential theft, evasion, unauthorized account actions, surprise
installation, or destructive cleanup without exact user intent and an approved
rollback/quarantine design.

Never use real personal, production, employer, or customer systems as mutation
test fixtures. Write-path testing belongs on disposable, snapshot-backed hosts
with synthetic data and explicit current approval.

## Release and supply-chain posture

There is no production signing key, package feed, installer, bootstrap command,
or supported update channel. Local ZIP/DMG builders and public-test-key module
fixtures are evidence only. Checksums, test signatures, ad-hoc signatures, and
keyless attestations must not be described as Apple notarization, Windows
Authenticode reputation, or production module trust.

Before the first supported release, the project still requires:

- fresh dependency/advisory/license review;
- production provenance, key custody/rotation/revocation, and compromised-
  release response;
- reproducible final artifacts and target-bound SBOM/notices;
- complete platform/module/lifecycle acceptance evidence;
- external security and unsafe-code review;
- supported-version, vulnerability-response, incident, and disclosure policy;
- honest unsigned-artifact warning and verification instructions for channels
  that do not use paid platform signing.

See [`docs/production-readiness.md`](docs/production-readiness.md) for the full
release gate and [`docs/free-release-distribution.md`](docs/free-release-distribution.md)
for the no-paid-signing policy.
