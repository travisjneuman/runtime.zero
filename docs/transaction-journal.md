# Transaction Journal and Recovery Contract

`crates/transaction-contract/` owns the schema-1 transaction state machine,
tamper-evident event chain, deterministic recovery assessment, and immutable
snapshot writer shared by all future mutating modules. It cannot authorize a
mutation or perform an action-plan write.

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
signature or trust root. The foundation writer durably publishes immutable
snapshot heads, but a committed action still must bind the final head into an
authenticated receipt. Schema-1 durability booleans require runtime evidence;
they are not made true merely by serialization.

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

## Durable immutable snapshot writer

`publish_journal_snapshot` and `recover_journal_head` provide the first reusable
write-capable transaction foundation. They:

- acquire a per-transaction cross-process advisory writer lock (`flock` on Unix
  and `LockFileEx` on Windows);
- require the first durable head to contain only `prepared` and every later head
  to append exactly one event;
- serialize at most 2 MiB under the shared resource contract;
- create a private pending file, synchronize it, and atomically rename it to an
  immutable sequence/event-digest-bound head;
- synchronize containing directories on Unix;
- reject symlink/reparse/hardlink/wrong-type roots, lock files, directories, and
  heads, with no-follow snapshot/lock opens;
- recover only after validating every bounded snapshot as one exact immutable
  prefix; corruption is never skipped;
- map durable-writer failures to the shared foundation machine-error vocabulary;
- make republishing an identical head idempotent.

Unix-created transaction directories are mode `0700` and snapshots/lock files
are mode `0600`. Windows uses inherited user-local ACLs pending explicit ACL
runtime evidence. Existing guarded OS-temp simulations remain separate fault
fixtures for interruption and corruption behavior.

## Commit receipt binding

`transaction_commit_receipt` binds one valid committed journal head to the exact
action-plan digest, write-set digest, confirmation challenge/response/consumption
digests, durable single-use-consumed state, prior registry digest (or verified
absence), and next registry digest. Its domain-separated binding digest also
commits to the journal snapshot name and required publication order:

1. synchronize the committed journal head;
2. synchronize the commit receipt;
3. atomically publish the registry last.

Tampering with any identity, head, plan, write set, confirmation evidence,
registry state, or ordering claim invalidates the receipt. Schema 1 explicitly sets
`automatic_mutation_authorized: false`; the receipt is evidence and never an
instruction to finish or repeat a write. Filesystem publication of receipt and
registry documents remains the next coordinator layer.

## Remaining production work

The complete store transaction still requires safe opened-root handles across
all path operations, Windows directory-metadata flush evidence, durable commit-
receipt publication, atomic installed-registry publication, explicit ACL/ownership
verification, cancellation, fault injection at every boundary, and real
power/process-loss recovery on Windows, macOS, and Linux. Quarantine and
rollback need platform-specific locked-file, reparse/symlink, cross-filesystem,
and metadata-fidelity proof. No module may implement a private journal,
writer-lock, or recovery engine.

See [`action-planning.md`](action-planning.md),
[`transaction-simulation.md`](transaction-simulation.md), and
[`production-readiness.md`](production-readiness.md).
