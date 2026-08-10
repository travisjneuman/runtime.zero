# Cache module

`rz0-module-cache` is a development-only ownership-aware classifier. It accepts
caller-supplied synthetic cache evidence and maps it into the shared
`classified_finding_report` contract.

Current policy:

- exact runtime.zero-owned cache evidence with exact digest/size may become a
  quarantine candidate;
- manager-, system-, or user-owned cache evidence remains report-only;
- unknown ownership or protected data is blocked;
- output is path-free, read-only, and non-authorizing.

The package has no binary, live filesystem adapter, manager access, network
access, process host, action-plan generator, quarantine/restore implementation,
or cleanup/delete path. Its manifest remains `planned`, and core neither
installs nor executes it.

Before 1.0 it needs bounded platform-specific discovery, ownership and active-use
proof, size/age/resource budgets, adversarial fixtures, finding-bound plans,
quarantine/restore transactions, cancellation/recovery, CLI/JSON/TUI, and every
Windows/macOS/Linux lifecycle acceptance cell. User/shared/unknown caches must
remain report-only unless the frozen policy is explicitly changed.
