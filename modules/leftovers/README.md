# Leftovers module

`rz0-module-leftovers` is a development-only post-uninstall classifier over
caller-supplied synthetic evidence. It maps records into the shared path-free
finding contract.

Current policy:

- only exact runtime.zero-owned orphan/executable evidence with digest and size
  can become a quarantine candidate;
- protected, credential/session, browser-profile, project/workspace, backup,
  user-content, shared, system, and unknown evidence is blocked or report-only
  according to foundation policy;
- classification never grants action authority.

The package does not scan filesystems, package receipts, shims, services,
launch entries, or PATH; it does not plan writes, quarantine, restore, or delete.
It has no binary/process protocol, live platform capability, signed lifecycle
artifact, or core integration. Its manifest remains `planned`.

Completion requires bounded post-uninstall adapters on every target, exact
ownership/provenance and stale-state proof, adversarial/partial-evidence
fixtures, finding-bound plans, receipt-scoped quarantine/restore, retention and
conflict policy, cancellation/recovery, CLI/JSON/TUI, and all release-ledger
cells. Broad recursive leftover scans remain out of scope.
