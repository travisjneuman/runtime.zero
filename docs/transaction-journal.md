# Transaction Journal and Recovery Contract

`crates/transaction-contract/` owns the schema-1 transaction state machine,
tamper-evident event chain, and deterministic recovery assessment shared by all
future mutating modules. It performs no filesystem I/O and cannot authorize a
mutation.

## Schema-1 journal

A `transaction_journal` binds:

- a normalized transaction ID and immutable action-plan ID;
- one foundation operation: module install/upgrade/repair/uninstall, manager
  update/uninstall, quarantine, or restore;
- a state derived from the final event;
- the mandatory durability posture: append-only events, sync after every event,
  atomic head publication, and receipt binding to the final head;
- between 1 and 1,024 contiguous events.

Events support prepared, apply started, exact write intent, verified write,
commit started/committed, recovery required, rollback started, verified rollback,
and rolled back states. Transitions are allowlisted. Terminal committed and
rolled-back states cannot be extended.

Write intent and verification events require an action ID, normalized relative
path, and exact expected post-write SHA-256. A verification must immediately
and exactly match its intent. The optional before digest records replacement
state; absence means the path was expected not to exist. Rollback verification
records the restored before digest, or no digest when verified rollback restores
absence.

## Deterministic event commitment

Each event SHA-256 uses an explicit domain-separated, length-prefixed encoding.
It binds the transaction ID, plan ID, operation, sequence, event kind, optional
action/path/evidence, and previous event digest. The first event points to the
all-zero digest. Validation rejects discontinuity, duplication, tampering,
header transplantation, impossible state transitions, unsafe paths, malformed
digests, and state/head disagreement.

This chain detects accidental or uncommitted modification; it is not a
signature or trust root. A production writer must durably publish the head and
bind it into an authenticated receipt. Schema-1 durability booleans are required
intent, not evidence that an operating system actually synchronized data.

## Recovery assessment

A valid journal produces one conservative decision:

- prepared: abort without writes;
- applying, commit-pending, or recovery-required: roll back verified writes;
- committed: verify committed state;
- rolling back: resume rollback;
- rolled back: no action.

An invalid journal produces `refuse_invalid_journal`. Every assessment sets
`automatic_mutation_authorized: false`; a decision describes required operator
or future policy handling and never executes it.

## Remaining production work

Production use still requires a store writer with safe root handles, create-new
and atomic-replace semantics, file and parent-directory synchronization,
exclusive transaction ownership, receipt/head publication ordering, ACL and
ownership policy, capacity limits, cancellation, fault injection at every
boundary, and real power/process-loss recovery on Windows, macOS, and Linux.
Quarantine and rollback need platform-specific locked-file, reparse/symlink,
cross-filesystem, and metadata-fidelity proof. No module may implement a private
journal or recovery engine.

See [`action-planning.md`](action-planning.md),
[`transaction-simulation.md`](transaction-simulation.md), and
[`production-readiness.md`](production-readiness.md).
