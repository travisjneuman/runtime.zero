# Cache management contract

`rz0 cache --dry-run` is a bounded evidence review, not a generic cache
cleaner. It inspects only the documented runtime.zero, Homebrew, npm, pip, and
Cargo roots on the current platform. It never scans a home directory, profile,
drive, PATH, registry, process table, network source, or package-manager
database to discover additional cache data.

## Ownership and disposition

- runtime.zero-owned cache files may receive an exact, digest- and size-bound
  quarantine plan when the user supplies one absolute file path and confirms a
  fresh challenge;
- manager-owned cache roots are report-only because the manager remains the
  owner of their retention semantics;
- user-, shared-, system-, and unknown-owned data is report-only or blocked;
- live directory observations never become action-ready file identities.

## Review budgets

Live evidence reports these conservative review signals:

- files at least 30 days old, based on bounded regular-file modification
  metadata;
- a 16 MiB runtime review-size budget for future runtime-owned candidate
  selection;
- aggregate scan ceilings of 2,048 entries and 64 MiB per known root.

The age and size values are review thresholds, not deletion or quarantine
authority. The exact runtime.zero file lane still requires fresh file digest and
size evidence, an unoccupied non-symlink destination, a short-lived challenge,
and the receipt-bound foundation executor.

## Active-use and exclusions

The adapter records possible `lock`, `.lock`, and `.lck` markers. A marker makes
active use possible; no marker does not prove inactivity because portable
process/native lock proof is not available in the bounded cross-platform
review. Incomplete scans, unreadable metadata, possible lock markers, links,
special files, directories, cross-root paths, and any active-use uncertainty
are excluded from future automatic action. No manager command, elevation,
recursive cleanup, deletion, network access, or hidden retry is permitted.

JSON includes the `cache_safety_policy` summary and path-free per-root
observations. The TUI Diagnostics workspace renders the same observation count,
age-threshold count, and uncertainty state; it does not create a second action
authority.

## Remaining release work

Public release still requires platform-native active-use/ownership proof,
multi-file retention and conflict policy, low-space/locked/concurrent-writer/
permission testing, metadata-preserving quarantine and restore, and the full
Windows/macOS/Linux acceptance matrix. Until those gates are complete, user,
shared, manager, and unknown cache data remains report-only.
