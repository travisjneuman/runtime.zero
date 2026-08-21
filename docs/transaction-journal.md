# Transaction Journal and Recovery Contract

`crates/transaction-contract/` owns the schema-1 transaction state machine,
tamper-evident event chain, deterministic recovery assessment, immutable
snapshot writer, and multi-document commit coordinator shared by all future
mutating modules. It cannot authorize a
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
- create a private pending file, synchronize it, and publish an immutable
  sequence/event-digest-bound head without replacement;
- request file and containing-directory synchronization;
- reject symlink/reparse/hardlink/wrong-type roots, lock files, directories, and
  heads, with opened-directory-relative Unix operations and compile-checked NT
  root-relative Windows operations;
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
instruction to finish or repeat a write.

## External manager effect receipt

A manager update is not an atomic runtime.zero-owned file publication, so
`external_effect_commit_receipt` records the exact verified external outcome
before final journal completion. It binds the transaction/action/manager/target,
commit-pending journal head, plan/write-set and confirmation digests, sealed
executable identity and spawn mechanism, argument-vector digest, bounded process
exit/output evidence, fresh post-action verification digest, rollback posture,
and `automatic_mutation_authorized: false`.

`publish_external_effect_receipt_cancellable` synchronizes a create-new receipt
before the updater appends its final committed journal head. Identical
republication is idempotent; malformed, duplicate-conflicting, mismatched, or
out-of-order evidence fails closed. `assess_external_effect_recovery` compares
immutable journal/receipt state and returns only a non-mutating decision: abort
without writes, verify an uncertain manager outcome, require future exact
approval for final journal completion, no action for consistent committed
state, or refuse inconsistent evidence.

The core exposes this assessment through
`rz0 updates --recovery-status --transaction <id>`. A verified
`complete_journal_commit_with_explicit_approval` assessment can be completed
with:

```bash
rz0 updates --recovery-complete --transaction <id>
rz0 updates --recovery-complete --transaction <id> \
  --challenge-issued-unix-seconds <issued> --confirm '<exact-phrase>'
```

The completion lane revalidates the exact receipt and commit-pending journal,
records a durable receipt-bound approval, and appends only the already
authorized final local journal event. It never reruns a manager, edits a
receipt, rolls back, or grants automatic mutation authority.

## Commit coordinator

`publish_confirmation_consumption` stores exact single-use evidence only after
verifying the prepared immutable journal, action-plan digest, write set,
capability set, risk, response, and transaction identity. Identical publication
is idempotent; conflicting consumption requires recovery.

`publish_committed_state` retains opened state/transaction/receipt directories,
takes an exclusive state commit lock, and revalidates the committed head,
consumption, receipt, canonical next registry, and exact prior registry bytes.
It then:

1. stores a synchronized rollback copy when prior registry state exists;
2. stores the synchronized canonical next registry as a pending document;
3. publishes the synchronized create-new receipt;
4. atomically replaces or initially publishes `installed-modules.json` last;
5. rereads and compares the final bytes.

Successful final state is idempotent. Any partial prior attempt returns
`recovery_required` rather than silently retrying. `assess_commit_recovery`
classifies exact no-action, interrupted-final-publication, uncommitted-pending,
or inconsistent states and always sets `automatic_mutation_authorized: false`.

The optional `fault-injection` feature interrupts deterministically after each
of eight commit boundaries: evidence validation, lock, durable evidence,
rollback backup, pending registry, receipt, registry publication, and final
verification. The same lane drives cancellation at every boundary. The
cancellable coordinator returns typed `cancelled` only before partial commit
publication, returns `recovery_required` after partial publication, and
preserves success after exact final verification. It never rolls back, cleans,
or retries because a signal arrived. Fault injection is test-only and not
enabled by the product.

Only the narrowly safe interrupted state—exact committed journal, durable
confirmation, exact receipt, exact pending registry, and unchanged prior
registry—can use `complete_interrupted_registry_publication`. Completion requires
a new five-minute assessment/receipt-bound interactive phrase, durably stores the
single-use recovery approval, and performs only the previously authorized final
registry publication. It cannot execute plan writes or rollback and still
returns `automatic_mutation_authorized: false`. The coordinator is not connected
to a production module executor or user command.

## Remaining production work

The updater consumes the canonical journal, confirmation publication, exact
write-intent/verification events, and external-effect receipt/recovery model.
The complete store/module transaction still requires reviewed Windows owner/DACL
privacy verification and directory-flush evidence, real process/power-loss fault
execution beyond deterministic local injection, rollback execution,
cancellation propagation through remaining process/write paths, and recovery
evidence on Windows, macOS, and Linux. The commit coordinator's eight
synchronized boundaries are cancellation-aware; this does not prove abrupt
process/power loss or other writers. The current
coordinator enforces Unix effective-user ownership and private permission bits
and deliberately blocks on Windows at that gate. Quarantine and
rollback need platform-specific locked-file, reparse/symlink, cross-filesystem,
and metadata-fidelity proof. No module may implement a private journal,
writer-lock, or recovery engine.

See [`action-planning.md`](action-planning.md),
[`installed-registry-contract.md`](installed-registry-contract.md),
[`transaction-simulation.md`](transaction-simulation.md), and
[`production-readiness.md`](production-readiness.md).
