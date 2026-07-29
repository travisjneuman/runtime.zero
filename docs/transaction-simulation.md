# Test-Only Transaction Simulation

`runtime.zero` exercises immutable staging, quarantine, and restore semantics in
integration tests before exposing any product mutation path. These simulations
are evidence for the contracts; they are not module installation or action
execution features.

## Isolation boundary

Filesystem-writing helpers exist only under crate `tests/` directories. Every
simulation root must:

- be a direct child of the canonical OS temporary directory;
- use an `rz0-transaction-sim-*` or `rz0-action-sim-*` prefix;
- contain the expected schema-1 simulation marker;
- keep every source, staging, publication, quarantine, record, and restore path
  normalized and relative to that root;
- reject symlinks in inspected source/root components;
- use create-new semantics and refuse occupied destinations;
- report `production_writes_attempted: false` where a simulation receipt is
  modeled;
- be removed only by the owning test's isolated-root cleanup.

No CLI or library production API invokes these helpers.

## Immutable package staging

A schema-1 staging plan remains `simulation_only: true`, `dry_run: true`, and
`writes_attempted: false`. It binds:

- transaction ID, first-party package ID/version, and manifest SHA-256;
- a successful test-key signature verification with matching identity/digest;
- source, unpublished staging, and publication roots;
- at most 128 normalized package files, each no larger than 64 MiB and no more
  than 512 MiB total;
- exactly one `rz0-module.json` manifest matching the signed digest;
- atomic same-root publication and preservation of failed unpublished stages.

The test helper reads each fixture file once into bounded memory, verifies its
size/digest, writes those exact bytes to a new staging path, verifies the copy,
and renames the complete stage to an unoccupied publication path. Tests cover
successful publication, tampered input, symlinked input, identity/path/proof
drift, existing publication conflicts, and retained partial staging for review.

## Quarantine and restore

Schema-1 quarantine/restore plans bind exact simulation-relative source paths,
SHA-256 digests, sizes, capabilities, write-set kinds, and rollback posture.
The test helper:

1. verifies the source and rejects symlinks or drift;
2. writes a new quarantine copy and verifies its digest;
3. supports an injected failure after copy, proving the original remains;
4. removes the original only after the verified copy exists;
5. records the fixture-relative original/quarantine paths and digest;
6. restores only to an unoccupied path using another verified create-new copy;
7. retains the quarantine payload after restore.

Tests cover round-trip restore, injected partial failure, tampered source,
symlinked source, and occupied restore destinations. Credentials, sessions,
browser profiles, projects, backups, shared data, and unknown data remain blocked
by the action-plan policy and are not simulation inputs.

## Shared journal contract

`crates/transaction-contract/` now defines the shared bounded state machine,
hash-chained write-intent/verification events, exclusive immutable snapshot
publication/recovery, and conservative recovery decisions. Every recovery
assessment explicitly refuses to authorize automatic mutation. Separate
simulation fixtures preserve the prior head when publication is interrupted and
exercise corruption/symlink rejection; the reusable writer additionally enforces
cross-process ownership and exact one-event durable prefixes. See
[`transaction-journal.md`](transaction-journal.md).

## Remaining production gates

The simulations do not establish crash durability, ACL/ownership fidelity,
locked-file handling, cross-filesystem behavior, Windows reparse semantics,
platform sandboxing, privileged operations, or safe recovery after process/power
loss. They add no production mover, installer, registry writer, cleanup command,
module execution, or permanent deletion path.

A fixture-only invocation/not-executed response protocol now defines exact
receipt binding, least-privilege read grants, and host/child I/O ceilings. An
explicit-feature lane exercises only a Cargo-built test helper. Before any
developer-only installed artifact trial, runtime.zero still needs executable-
handle pinning, process-tree/handle control, capability enforcement, real
platform isolation proof, production receipt/journal design, and explicit
review of any non-temporary filesystem mutation.
