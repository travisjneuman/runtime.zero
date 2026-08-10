# Exact Plan Confirmation Contract

`crates/confirmation-contract/` owns interactive confirmation semantics for the
current core updater and all future mutating modules. A module cannot invent a
weaker `yes/no` prompt or interpret confirmation as execution authority.

## Challenge binding

A schema-1 challenge binds:

- exact plan, completed dry-run, write-set, before-state, and expected-after-state
  SHA-256 values;
- plan/challenge identity, risk, action count, and the unique sorted action
  capabilities;
- issue and expiration time with a maximum five-minute lifetime;
- proof that dry-run completed without writes;
- an explicit rollback or quarantine story, or a separately displayed manual-
  recovery acknowledgement for an action whose native rollback is unavailable;
- a domain-separated challenge digest and plan-specific phrase.

`rz0-action-plan` provides deterministic domain-separated plan and write-set
digests only after the complete plan validates. Presentation-only plan changes
alter the plan digest; write-set changes alter both digests.

## Interactive response

Schema 1 accepts only CLI or TUI responses marked interactive and single-use.
The operator must enter the complete generated phrase, including the plan ID and
a challenge-digest prefix, during the validity window. Generic `--yes`, stale
phrases, mismatched previews, future timestamps, non-interactive responses, and
fabricated execution authority fail closed.

A valid response means only `plan_confirmed: true`. Every assessment retains
`execution_authorized: false` and requires durable consumption before any
transaction write.

## Replay prevention

`plan_confirmation_consumption` binds one response digest to one transaction and
plan. Its domain-separated digest is intended for create-new publication at a
response-digest-derived filename. Reusing the response must conflict with the
existing consumption record. The transaction commit receipt binds challenge,
response, and consumption digests and requires consumption evidence.

The contract does not itself approve elevation, grant a capability, or mutate
a target. The core updater now uses the durable consumption coordinator before
an explicit manager apply; module invocation and other domain mutation lanes
remain separately gated. Durable create-new consumption belongs in the
foundation transaction coordinator, never in a module.

See [`action-planning.md`](action-planning.md),
[`transaction-journal.md`](transaction-journal.md), and
[`production-readiness.md`](production-readiness.md).
