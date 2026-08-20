# Cache module

`rz0-module-cache` is the ownership-aware cache classifier used by the
read-only `rz0 cache --dry-run` foundation surface. It maps bounded local or
caller-supplied evidence into the shared `classified_finding_report` contract;
the core wraps that result in the `cache_review` CLI/JSON contract.

Current policy:

- exact runtime.zero-owned cache evidence with exact digest/size may become a
  quarantine candidate;
- manager-, system-, or user-owned cache evidence remains report-only;
- unknown ownership or protected data is blocked;
- output is path-free, read-only, and non-authorizing.

The core live adapter inspects only bounded known roots for runtime.zero,
Homebrew, npm, pip, and Cargo. It rejects root symlinks, skips descendant
symlinks and special files, caps entries and aggregate bytes, deduplicates
warnings, and emits directory-listing evidence without treating it as an
action-ready file identity. The fixture mode accepts one strict local JSON
document for deterministic support/testing. Neither mode writes, invokes a
manager, contacts a network source, or authorizes cleanup.

Before 1.0 it needs stronger platform-specific ownership and active-use proof,
age/resource policy, finding-bound plans, quarantine/restore transactions,
cancellation/recovery, and every Windows/macOS/Linux lifecycle acceptance cell.
User/shared/unknown caches must remain report-only unless the frozen policy is
explicitly changed. The TUI Diagnostics workspace shows the same bounded
observation count and warning state; it does not add a second action path.
