# Security and integrity module

`rz0-module-security-integrity` is an evidence classifier for caller-supplied
exact digest observations, exposed through the strict fixture-only foundation
surface `rz0 integrity --dry-run --fixture ...`. Matches and mismatches map
into the shared path-free finding contract; mismatches can be high-risk
evidence but remain report-only. Unknown ownership is blocked.

The package and core fixture surface do not scan the host, establish a trusted
baseline, claim malware or vulnerability detection, contact a network source,
remediate, quarantine, restore, or execute anything. They have no live
baseline permission, action-plan path, signed lifecycle artifact, or active
module lifecycle. Its manifest remains `planned`.

Production completion requires explicit checks/non-goals, versioned trusted
baseline provenance and revocation, bounded live adapters, false-positive and
incident policy, privacy review, platform/runtime fixtures, CLI/JSON/TUI,
accessibility/support documentation, and all Windows/macOS/Linux release cells.
Remediation is outside the current module and would require its own complete
finding/action/confirmation/transaction/rollback scope.
