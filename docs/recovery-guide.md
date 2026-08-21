# Recovery Guide

Recovery is evidence-led and fail-closed. Do not manually delete transaction
directories, edit JSON, rename receipts, rerun a manager command, or repeat a
confirmation phrase to make an error disappear. Preserve the private state root
until exact evidence is understood.

This guide covers current behavior. It does not claim production power-loss or
rollback support.

## First response

1. Stop initiating new write actions.
2. Record the command, transaction ID, exit status, and exact redacted error.
3. Do not include the state root, software names, process output, or private
   paths in a public issue.
4. Run read-only diagnostics and inventory:

   ```bash
   rz0 doctor --format json
   rz0 report --format json
   ```

5. For an updater transaction, run:

   ```bash
   rz0 updates --recovery-status --transaction <exact-id> --format json
   ```

6. Preserve the state root and manager-native history/logs. Review them locally;
   do not publish raw evidence.

For runtime.zero-owned quarantine evidence, use the bounded read-only inventory
before considering an exact restore:

```bash
rz0 recovery --dry-run --format json
```

This command does not repair malformed records or remove stale payloads. If a
record is valid and restore-capable, use the separate exact `rz0 restore` flow
described in [`user-guide.md`](user-guide.md); occupied, drifted, symlinked, or
unsupported destinations remain blocked.

The same review now inspects the private transaction-root journal directories
through immutable snapshot heads. JSON and text output report the bounded number
of checked, valid, invalid, and action-required journals, plus a stable
per-transaction decision and operator guidance. Journal IDs, plan IDs, states,
and decisions are logical evidence; absolute state-root paths and raw journal
details are not exposed.

Journal inspection is strictly report-only. It does not acquire a writer lock,
create a lock file, publish a snapshot, complete a transaction, restore a
payload, or authorize rollback. Persistent writer-lock markers are counted as
evidence, but their presence alone does not prove that a writer is active; this
read-only review does not determine lock ownership. A concurrent publication
can likewise produce incomplete evidence, so preserve the state and repeat the
review after writers have stopped when that can be established independently.
It is never treated as permission to mutate.

## Durable transaction states

| State | Meaning | Current safe response |
| --- | --- | --- |
| `prepared` | Plan and initial journal exist; no apply event is durable | Do not retry blindly. Current recovery assessment can classify abort-without-writes but does not delete evidence. |
| `applying` | External action may have started; exact outcome may be unknown | Inspect fresh manager-installed and update-availability state. Do not rerun. |
| `commit_pending` | Verification completed and local finalization started | Check for the exact external-effect receipt. Missing or conflicting evidence requires review. |
| `committed` | Final journal event is durable | Verify the receipt and fresh system state agree; no automatic follow-up is implied. |
| `recovery_required` | A failure/cancellation occurred after a write boundary or evidence conflicted | Stop. Follow the assessment and preserve all evidence. |
| `rolling_back` / `rolled_back` | Reserved canonical rollback states | No production updater rollback executor currently emits these states. Do not fabricate them. |

## External manager effect receipt

Manager writes cannot be atomically committed with runtime.zero files. The
`external_effect_commit_receipt` therefore binds:

- the exact transaction, action plan, manager, target, and commit-pending journal
  head;
- plan/write-set and confirmation challenge/response/consumption digests;
- the sealed executable SHA-256, size, and identity-to-spawn mechanism;
- a digest of the exact argument vector;
- bounded exit/output counts and output digests;
- a digest of fresh post-action verification;
- rollback posture and explicit non-authorization of automatic mutation.

The receipt is synchronized before the final `committed` journal head. If the
process stops between those writes, recovery can distinguish a verified effect
from an unknown manager outcome. The receipt does not authorize a rerun,
rollback, or journal edit.

## Recovery-status decisions

### `abort_without_writes`

Only prepared evidence exists and no manager apply event is durable. Preserve
the transaction. A future evidence-retention operation may archive it, but no
current command removes it automatically.

### `verify_external_effect`

The manager may have run but no valid verified receipt closes the outcome.
Inspect the manager directly using its read-only installed/availability
interfaces. Compare exact package/source/version identity. Do not infer failure
from a missing runtime.zero receipt and do not invoke update again.

### `complete_journal_commit_with_explicit_approval`

A valid receipt binds the exact commit-pending prefix, but final journal state is
incomplete. The product provides a narrow command that can complete only this
local state after fresh explicit approval. Request a challenge with:

```bash
rz0 updates --recovery-complete --transaction <exact-id> --format text
```

Then repeat with the exact issued timestamp and phrase from that challenge:

```bash
rz0 updates --recovery-complete --transaction <exact-id> \
  --challenge-issued-unix-seconds <issued> --confirm '<exact-phrase>'
```

The
completion records a durable receipt-bound approval and appends only the
previously authorized final local journal event. It never reruns the manager,
edits a receipt, rolls back, or grants automatic mutation authority.

### `no_action`

The receipt validates against the committed journal prefix. This means local
evidence is internally consistent, not that broader system health or rollback
has been proven. Fresh inventory may still be appropriate.

### `refuse_inconsistent_evidence`

Receipt/journal state is malformed, mismatched, missing in an impossible order,
or otherwise unsafe. Preserve it. Do not repair by hand.

## Cancellation semantics

On Unix, the updater installs a temporary SIGINT bridge only around the confirmed
execution. The first interrupt becomes typed `user_requested` cancellation.
The process host terminates/reaps its dedicated process group and the updater
publishes `recovery_required` where possible.

Limitations:

- a child that successfully escapes its process group is not contained;
- a signal cannot reverse an already completed manager action;
- cancellation during post-action verification can leave outcome review
  required;
- abrupt kill, kernel panic, reboot, or power loss is not equivalent to orderly
  SIGINT;
- Windows runtime cancellation/containment proof, broader capability isolation,
  and macOS identity-bound spawn remain incomplete.

## Store initialization recovery

Store initialization creates runtime.zero-owned paths and files in deterministic
order with create-new semantics. A rerun inspects every path and can continue
from safe matching partial scaffolding. It refuses:

- symlink/reparse or wrong-type paths;
- invalid existing registry JSON;
- an invalid store-init marker;
- unsupported platform mutation.

If initialization stops partway through, run `rz0 store init --dry-run` and
review every step. Do not remove a path unless it is proven runtime.zero-owned
from local evidence. The marker's rollback text is guidance, not an automatic
uninstaller.

## Module/registry recovery

The canonical module commit coordinator has stronger registry-last simulation
and explicit final-publication recovery contracts, but no production module
lifecycle executor calls them. Do not use module transaction fixtures to alter a
real store.

## Evidence handling

- Keep state permissions private.
- Prefer `rz0 report` for support; it omits raw transaction evidence.
- Never paste confirmation phrases, private paths, process output, application
  names, service labels, credentials, or state files into a public issue.
- Hash a private evidence copy if transfer is explicitly approved; do not send
  it by default.
- Treat a successful parser/validator as integrity evidence, not authenticity.

## Still required for production recovery

- manager-specific recovery completion beyond the verified local journal event;
- manager-native rollback or reviewed manual recovery per manager/version;
- post-reboot and power-loss proof on disposable hosts;
- Windows owner/DACL/directory-flush and process-tree proof;
- macOS exact executable binding and sandbox constraints;
- locked-file, low-space, read-only, concurrent-writer, corrupt-disk, and
  cross-filesystem matrices;
- detailed per-transaction TUI recovery review and independent security review.
