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
- live review reports files older than 30 days and a 16 MiB runtime review-size
  budget as evidence only; these thresholds do not authorize cleanup;
- active use is conservatively represented by possible `lock`/`.lock`/`.lck`
  markers, while no marker still means active use is unknown;
- output is path-free, read-only, and non-authorizing.

The core live adapter inspects only bounded known roots for runtime.zero,
Homebrew, npm, pip, and Cargo. It rejects root symlinks, skips descendant
symlinks and special files, caps entries and aggregate bytes, deduplicates
warnings, and emits directory-listing evidence without treating it as an
action-ready file identity. The fixture mode accepts one strict local JSON
document for deterministic support/testing. The classifier never writes,
invokes a manager, contacts a network source, or grants cleanup authority. The
separate core CLI lane can plan and, after exact confirmation, quarantine one
explicit regular file inside the runtime.zero cache root through the foundation
receipt/journal executor; it never recurses or deletes. `rz0 restore` can later
restore that one validated quarantine record to its original unoccupied cache
path after a fresh exact confirmation; it is not cache discovery or retention.

Before 1.0 it needs stronger platform-specific ownership and active-use proof,
platform-native age/resource policy, retention and conflict policy, full
multi-file quarantine/restore transactions, cancellation/recovery, platform
metadata fidelity, and every Windows/macOS/Linux lifecycle acceptance cell.
User/shared/unknown caches must remain report-only unless the frozen policy is
explicitly changed. The TUI Diagnostics workspace shows the same bounded
observation count, age-threshold count, and warning state; it does not add a
second action path.
