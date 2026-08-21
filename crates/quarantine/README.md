# Quarantine foundation

rz0-quarantine is the foundation-owned executor for the narrow quarantine and
restore action contract. It is intentionally not a general cleanup API.

The executor requires:

- one validated, single-action ActionPlan;
- an exact plan-bound confirmation challenge, response, and durable
  ConfirmationConsumption;
- explicit absolute source, quarantine, and private state roots;
- exact source digest/size evidence and a matching quarantine record for
  restore;
- destination creation without replacement.

It uses opened-directory-relative secure filesystem operations, an exclusive
quarantine lock, append-only transaction journal snapshots, a tamper-evident
quarantine record, and a filesystem-effect receipt. Symlinks, unsafe path
components, source drift, occupied destinations, missing records, and invalid
confirmation fail closed.

The crate does not discover candidates, decide ownership, create confirmation
phrases, authorize module execution, perform recursive deletion, or expose a
public cleanup command. Domain modules and the CLI/TUI must continue to route
through the shared finding and action-plan contracts before this executor is
used on a real runtime.zero-owned path. Cross-filesystem moves and broader
platform-specific bundle semantics remain explicit follow-up work.
