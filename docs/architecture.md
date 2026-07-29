# runtime.zero Architecture

`runtime.zero` is a modular system-management runtime, not a monolithic cleaner script. The core is the smallest durable foundation that can describe, validate, list, and eventually run explicitly installed modules under safety policy.

## Layers

1. **CLI core** — argument parsing, brand metadata, output, exit codes, and future interactive flows.
2. **Module registry** — manifest model, local manifest validation, installed-module listing, and core-vs-module reporting.
3. **Policy engine** — safety posture, deny rules, confirmation requirements, and mutation gates.
4. **Action planner** — future conversion of discoveries into update, uninstall, scan, quarantine, or restore plans.
5. **Platform adapters** — Windows, macOS, and Linux-specific discovery and execution primitives.
6. **Modules** — separately distributed capabilities that run on top of the foundation only after explicit installation/use.
7. **Quarantine/restore** — future timestamped local quarantine with manifests instead of hard delete by default.

## Foundation boundary

The core may include self-description, `doctor`, safe dry-run scaffolding, manifest schemas, output contracts, and policy primitives. It must not bundle substantial feature modules by default. First-party modules should be optional packages with declared capabilities, risk level, supported platforms, and safety behavior. Third-party modules require a separate trust model before implementation.

Local manifest loading is read-only and declarative. Loading a manifest means
parsing and validating JSON metadata; it does not load code, fetch dependencies,
install anything, enable anything, or run module entry points.

## Initial platform intent

Windows is the first practical target because the original need came from a Windows CLI/tool-manager workflow. The Rust core is cross-platform from day one so macOS and Linux adapters can be added without a rewrite.

## Inventory contract boundary

Core `rz0 scan --dry-run --format json` emits the versioned, empty
`inventory_report`. The first feature implementation is a separate workspace
package under `modules/inventory/`; the core does not depend on, install, load,
or execute it.

A small `crates/inventory-contract/` library owns the shared serializable model
so the feature package does not depend on the CLI/TUI core or pull its terminal
stack into the module binary. The module uses deterministic fixtures, bounded
process-PATH collection on
Windows/macOS/Linux, read-only persisted PATH and optional app registry reads on
Windows, bounded opt-in `.app`/XDG desktop-entry evidence on macOS/Linux,
allowlisted direct executable discovery, opt-in timeout-bounded version probes,
report-local path redaction, and structured source events. It does not invoke
package managers, execute desktop entries, inspect macOS bundle contents, or
recursively scan drives. See
[`inventory-schema.md`](inventory-schema.md).

`crates/capability-contract/` owns the shared read/action capability vocabulary
used by manifests, protocols, and plans while granting no authority.
`crates/error-contract/` owns stable machine codes and conservative privacy/retry
semantics; human messages are not policy. `crates/resource-contract/` owns
shared byte/record/timeout/process ceilings so modules can narrow but not expand
foundation budgets.
`crates/artifact-identity/` opens, bounds, hashes, identifies, revalidates, and
returns the same receipt-relative file handle without execution; platform
execution binding remains a later gate. `crates/module-trust/` supplies a
test-key-only detached Ed25519 verification contract without adding a signer,
key store, installer, execution path, or production trust root. Schema-1
staging plans and OS-temp integration tests also exercise immutable publication
and quarantine/restore failure semantics without a production mover.
`crates/transaction-contract/` owns the bounded hash-chained transaction state
machine and non-authorizing recovery assessment; durable storage remains gated.
`crates/module-protocol/` validates a read-only/offline
invocation preview and not-executed response with exact-path/digest metadata,
least-privilege grants, a cleared environment-name allowlist, and bounded I/O.
An explicit-feature integration lane launches only a Cargo-built test helper from
a guarded OS-temp receipt-like path to exercise framing, concurrent drains,
Unix inheritable-descriptor refusal, Unix process-group teardown, and compile-
only Windows Job Object tree teardown; no core or inventory-module launch
exists. Future module
execution, production signing, capability enforcement, durable transactions,
and distribution work is gated by
[`module-trust-and-execution.md`](module-trust-and-execution.md).
Update/uninstall/quarantine semantics are gated by
[`action-planning.md`](action-planning.md). Equal Windows/macOS/Linux release
requirements, the frozen module catalog, and the rule that shared stability,
security, transaction, observability, and efficiency behavior belongs in the
foundation are defined by
[`production-readiness.md`](production-readiness.md).

## Non-goals for Phase 1

- no update execution;
- no uninstall execution;
- no file cleanup;
- no malware claims;
- no Cloudflare deployment automation;
- no GitHub Actions;
- no package publishing.
- no remote module execution;
- no public direct-run bootstrap command until checksum/signing/release safety is designed.
