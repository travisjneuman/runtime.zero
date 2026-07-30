# Cancellation and Deadline Contract

`crates/cancellation-contract/` owns the allocation-minimal cancellation signal
used by foundation process and transaction consumers.

A cancellation pair allocates one shared `Arc<AtomicU8>`. Cloning a controller or
token clones only that shared pointer. The first cancellation reason wins through
one acquire/release compare-and-exchange and can never be overwritten:

- user requested;
- monotonic deadline exceeded;
- parent cancelled;
- host shutdown.

`ProcessDeadline` accepts caller-supplied monotonic millisecond ticks, rejects
zero/over-ceiling timeouts and arithmetic overflow, and never reads wall-clock
time. Polling an elapsed deadline atomically records the deadline reason unless
an earlier cancellation already won. Reasons map to the shared `cancelled` or
`timed_out` machine errors.

The explicit-feature module transport consumes this primitive for timeout
polling while preserving whole-process-tree teardown. The durable commit
coordinator also has a cancellable entry point and observes the token only at
synchronized transaction boundaries:

- cancellation before rollback/pending/receipt/registry publication returns the
  typed shared `cancelled` error;
- cancellation after any partial commit publication returns
  `recovery_required`, never cancellation-only success or automatic cleanup;
- cancellation after exact final-registry verification does not rewrite a
  completed commit as failure.

All eight coordinator boundaries have deterministic all-feature cancellation
classification tests. A token is a signal, not permission to spawn, kill,
mutate, retry, rollback, or recover. Production process hosts must still pair it
with platform tree containment and deterministic reap evidence, and other write
paths need the same boundary-specific integration.
