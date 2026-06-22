# runtime.zero Architecture

`runtime.zero` is a modular system-management runtime, not a monolithic cleaner script.

## Layers

1. **CLI core** — argument parsing, brand metadata, output, exit codes, and future interactive flows.
2. **Policy engine** — safety posture, deny rules, confirmation requirements, and mutation gates.
3. **Action planner** — converts discoveries into update, uninstall, scan, quarantine, or restore plans.
4. **Platform adapters** — Windows, macOS, and Linux-specific discovery and execution primitives.
5. **Modules** — individually scoped capabilities that run on top of the foundation.
6. **Quarantine/restore** — future timestamped local quarantine with manifests instead of hard delete by default.

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
