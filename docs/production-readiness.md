# Production Readiness and Completion Matrix

`runtime.zero` is pre-alpha. This document defines the finite completion model
that must replace vague claims such as “finish the full build” or “100%
complete.” Production completion means every required acceptance cell for the
frozen `1.0` scope has evidence; it does not mean the software will never evolve.

Windows, macOS, and Linux are equal release-blocking platforms. Every named 1.0
module is equally required. Compile-only evidence, fixtures, a successful run on
another platform, or a planned manifest cannot satisfy a platform runtime cell.

## Foundation ownership

Anything required to make multiple modules safe, stable, efficient, or
consistent belongs in the foundation rather than being reimplemented by a
module. The foundation owns:

- CLI, JSON, TUI, configuration, diagnostics, logging, and error taxonomy;
- versioned schemas, compatibility, migrations, and deprecation policy;
- store, registry, package integrity, trust, signatures, provenance, and
  revocation;
- process hosting, executable identity, platform isolation, capabilities,
  elevation, cancellation, concurrency, and resource limits;
- action-plan validation, confirmation, transactions, journals, receipts,
  quarantine, rollback, recovery, and post-action verification;
- platform abstraction boundaries and normalized evidence/action types;
- privacy/redaction, sensitive-data deny policy, audit events, and support
  evidence;
- shared performance budgets, bounded I/O, deterministic output, test harnesses,
  and release gates.

The shared capability vocabulary is centralized in
`crates/capability-contract/`; each versioned manifest/protocol/action schema
accepts only its exact subset, and classification never grants authority.

A module owns only its intended domain logic. It may collect domain evidence,
normalize findings, and propose or perform its exact approved action through
foundation services. It must not invent its own trust root, process host,
transaction format, confirmation flow, logger, updater, store, sandbox, or
rollback engine.

## Frozen 1.0 module catalog

The currently named functional module families form the initial 1.0 catalog:

1. **Inventory and environment** — PATH, tools, applications, packages,
   services, persistence, and normalized system evidence.
2. **Updater** — installed-only availability checks and exact manager-native
   update plans/execution.
3. **Uninstall** — exact manager-native uninstall planning/execution with
   shared-component and dependent-package review.
4. **Leftovers** — bounded post-uninstall ownership evidence, stale shims, and
   conservative quarantine candidates.
5. **Cache management** — bounded ownership-aware cache evidence and approved
   quarantine/cleanup.
6. **Security and integrity** — evidence-based integrity/security checks without
   malware-removal or unsupported assurance claims.
7. **Report and export** — privacy-reviewed deterministic text/JSON exports and
   support bundles without credentials or sensitive raw paths by default.

Module installation, trust, activation, capability enforcement, lifecycle, and
self/module management remain foundation responsibilities. A future commercial
or third-party module is not part of 1.0 until its exact function is named and
added to this matrix; once added, it receives the same platform and lifecycle
requirements as every other module.

## Equal platform matrix

The final supported OS-version, architecture, terminal, package-manager, and
installer sets must be frozen before release. Current broad targets are:

| Platform | Required production scope | Current evidence |
| --- | --- | --- |
| Windows | Supported Windows 10/11 and Server releases; architecture set to be frozen; registry, reparse, ACL, locked-file, handle, Job Object, service, package-manager, installer, terminal, recovery, and elevation behavior | Target compilation, fixture contracts, and compile-only test Job Object support; real runtime matrix incomplete |
| macOS | Supported current Intel/Apple Silicon releases; bundle, launch/service, package-manager, code identity, sandbox, ACL, filesystem, terminal, installer/notarization, recovery, and privilege behavior | Native inventory and guarded Unix test-helper evidence; production runtime incomplete |
| Linux | Supported distribution/kernel/libc/architecture set to be frozen; XDG, package/service/persistence managers, namespace/seccomp/landlock availability, filesystem, terminal, package formats, recovery, and privilege behavior | Target compilation and fixtures; real runtime matrix incomplete |

No platform may ship a module merely because another platform passed. A feature
may be explicitly unsupported only when the frozen 1.0 scope says so before
release, the CLI/JSON/TUI reports that state honestly, and the unsupported state
is tested.

## Module implementation matrix

Every module must reach the same lifecycle bar on every supported platform:

| Module | Windows | macOS | Linux | Current maturity |
| --- | --- | --- | --- | --- |
| Inventory/environment | Required | Required | Required | Read-only source implementation; package/service/persistence and runtime parity incomplete |
| Updater | Required | Required | Required | Fixture-only planning contract |
| Uninstall | Required | Required | Required | Fixture-only planning contract |
| Leftovers | Required | Required | Required | Policy and quarantine fixtures only |
| Cache management | Required | Required | Required | Named family only |
| Security/integrity | Required | Required | Required | Named family only |
| Report/export | Required | Required | Required | Named family only |

## Required lifecycle for every module-platform cell

A cell is complete only when all applicable stages have reviewed implementation
and runtime evidence:

1. Requirements, non-goals, threat model, privacy classification, and supported
   manager/source/root matrix.
2. Synthetic valid, missing, duplicate, malformed, adversarial, oversized,
   symlink/reparse, permission, locale, and partial-failure fixtures.
3. Bounded discovery with provenance, deterministic normalization, redaction,
   and useful unsupported/unavailable reporting.
4. Finding and action-plan schemas with immutable evidence digests, exact
   capabilities, risk, expiry, drift invalidation, and expected before/after
   state.
5. Text, JSON, and TUI review surfaces that never imply execution or support
   beyond evidence.
6. Exact foundation-enforced capability grant and platform isolation.
7. Explicit confirmation for the exact action, target, manager, network,
   privilege, and write set.
8. Durable transaction, journal, receipt, quarantine, rollback, cancellation,
   and interrupted recovery where mutation applies.
9. Post-action re-inventory and mismatch handling.
10. Repair, migration, upgrade, deactivation, and uninstall behavior where the
    module or its data has a lifecycle.
11. Unit, integration, end-to-end, property/fuzz, race, fault-injection,
    performance, resource, and soak evidence.
12. Security, privacy, accessibility, compatibility, documentation, support,
    and release review.

Read-only modules may mark mutation-specific stages not applicable, but only
through an explicit schema state and test. They still require trust, capability,
isolation, resource, privacy, UX, and release evidence.

## Foundation production workstreams

These workstreams are dependency-ordered. Platform-specific work proceeds in
parallel once its shared foundation dependency is stable.

1. Freeze 1.0 requirements, OS/architecture/manager/install-channel tables,
   schemas, compatibility policy, acceptance IDs, and measurable budgets.
2. Stabilize the core package/module/store/configuration/error/logging contracts
   and migration rules.
3. Close package and executable identity races; the same-open-handle artifact
   identity primitive and Unix root-relative no-follow traversal exist, but
   Windows root-handle semantics and platform execution binding remain.
   Implement production signatures, key policy,
   provenance, freshness, transparency, and revocation.
4. Implement platform process/handle/tree containment, capability enforcement,
   sandbox/elevation policy, network policy, cancellation, and resource limits.
5. Implement crash-safe staging, journals, receipts, atomic state, quarantine,
   rollback, idempotency, and interrupted recovery.
6. Implement foundation-owned module install, activation, invocation,
   deactivation, repair, migration, upgrade, and uninstall.
7. Complete inventory/environment parity on all platforms; use its normalized
   evidence as the prerequisite for mutating modules.
8. Implement updater, uninstall, leftovers, cache, security/integrity, and
   report/export modules against foundation APIs, maintaining platform parity.
9. Complete CLI/JSON/TUI action review, accessibility, terminal compatibility,
   help, manual pages, completions, recovery UX, and support diagnostics.
10. Complete dependency/supply-chain review, unsafe-code review, fuzzing,
    performance/soak/fault testing, external security review, and release audit.
11. Produce reproducible signed/notarized artifacts, SBOMs/notices, installers,
    package channels, offline paths, updates, rollback, and compromised-release
    response for every platform.
12. Complete beta/RC runtime matrices, documentation/site/brand parity,
    vulnerability response, support runbooks, go-live criteria, and rollback.

## Production execution gate

`crates/module-protocol/` owns a schema-1
`production_execution_assessment`. It enumerates the canonical artifact,
capability, executable-identity, process, runtime, and transaction gates for one
module/platform assessment. Schema 1 can report evidence and unresolved gates,
but it can never authorize product execution. Test-child evidence is never
silently promoted to production proof.

A later authorization schema must not be introduced until the mechanism and
runtime evidence for every gate are independently reviewed on each target
platform. The absence of a production host remains a code-level safety property,
not just documentation.

## Stability and efficiency requirements

Optimization means bounded, measured behavior rather than premature complexity:

- deterministic contracts and stable field semantics;
- no unbounded recursion, drive scans, output, input, concurrency, retries,
  memory, disk, network, logs, quarantine, or retention;
- manager-native operations and OS-native primitives before custom mutation;
- zero shell/PATH execution for security-sensitive commands;
- cancellation and timeouts that leave reconciled durable state;
- no duplicate terminal, trust, transaction, logging, or platform stacks across
  modules;
- benchmarks and resource budgets for startup, scan, memory, output, TUI render,
  transaction, and recovery paths;
- profiling before optimization and regression thresholds after it;
- useful partial evidence instead of global failure when independent sources are
  unavailable;
- no telemetry, network, daemon, service, or recurring work by default.

## Release evidence and definition of complete

`runtime.zero 1.0` is production-complete only when:

- the exact platform/module/manager/channel scope is frozen;
- every required matrix cell and lifecycle stage has a stable acceptance ID;
- every acceptance ID has implementation, automated evidence, real-platform
  runtime evidence, documentation, security/privacy review, and rollback or an
  approved not-applicable result;
- all production execution assessments are satisfied under a separately
  reviewed authorization schema;
- reproducible release artifacts, signatures, installers, updates, recovery,
  incident response, and support are exercised;
- public claims match the shipped behavior;
- the final cross-platform secret, safety, compatibility, accessibility,
  performance, recovery, supply-chain, and remote-artifact audit passes.

Production keys, publishing, deployment, external accounts, paid services,
recurring automation, third-party execution, host provisioning, and real
system mutation remain separately scoped consequential actions even though they
appear in the completion plan.
