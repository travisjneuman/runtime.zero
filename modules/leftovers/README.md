# Leftovers module

`rz0-module-leftovers` is the ownership-aware classifier used by the read-only
`rz0 leftovers --dry-run` foundation surface. It maps bounded runtime.zero-owned
module/log evidence or caller-supplied fixture evidence into the shared
path-free finding contract.

Current policy:

- only exact runtime.zero-owned orphan/executable evidence with digest and size
  can become a quarantine candidate;
- protected, credential/session, browser-profile, project/workspace, backup,
  user-content, shared, system, and unknown evidence is blocked or report-only
  according to foundation policy;
- classification never grants action authority.

The core live adapter inspects only runtime.zero's own module and log roots. It
rejects root symlinks, skips descendant symlinks and special files, caps entries
and aggregate bytes, deduplicates warnings, and emits directory-listing
evidence without treating it as an action-ready file identity. It does not scan
package receipts, shims, services, launch entries, or PATH; neither live nor
fixture mode writes, plans, quarantines, restores, or deletes anything. Its
manifest remains `planned`.

Before 1.0 it needs exact ownership/provenance and stale-state proof,
adversarial/partial-evidence fixtures, finding-bound plans, receipt-scoped
quarantine/restore, retention and conflict policy, cancellation/recovery,
platform parity, and all release-ledger cells. Broad recursive leftover scans
remain out of scope. The TUI Diagnostics workspace shows the same bounded
observation and warning state; it does not add a second action path.
