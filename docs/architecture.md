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
