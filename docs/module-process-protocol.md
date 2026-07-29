# Module Process Protocol Preview

`crates/module-protocol/` defines a versioned, fixture-only host/module process
contract without launching a process. Schema version `1` authorizes nothing: a
valid plan is read-only, dry-run, offline, path-redacted, and explicitly sets
both `execution_authorized` and `execution_attempted` to `false`.

## Invocation plan

The schema binds:

- request ID and exact `first-party.inventory` identity/version;
- target platform and the single `collect_inventory` operation;
- a receipt-relative `bin/` executable path and lowercase SHA-256;
- successful public-test-key metadata and exact manifest SHA-256;
- a cleared, non-inherited environment with names (not values) selected from a
  platform allowlist;
- timeout and stdin/stdout/stderr byte ceilings;
- unique, deterministically sorted exact read-only capabilities;
- explicit app/probe options and mandatory path redaction;
- mutation/network/execution authorization set to false.

Schema-1 ceilings are 10 seconds, 64 KiB stdin, 1 MiB stdout, and 64 KiB stderr.
The executable must come from a future verified receipt; no PATH lookup, shell,
URL, absolute path, traversal, or caller-supplied executable location is valid.
The model carries no environment values, secrets, tokens, cookies, command-line
arguments, arbitrary JSON, or filesystem write request.

## Least-privilege validation

Every plan requires process-environment and filesystem-metadata reads. Platform
and option grants must match exactly:

- Windows requires persisted-environment registry read; opt-in apps require
  application-registry read.
- macOS/Linux app inventory requires application-filesystem read and forbids
  Windows registry grants.
- Opt-in version probes require exact-command-probe permission.
- App/probe permissions are rejected when their option is not requested.

The environment is cleared rather than inherited wholesale. Current name-only
allowlists are:

- Windows: required `PATH` and `SystemRoot`, optional `WINDIR`;
- macOS: `HOME` and `PATH`;
- Linux: required `HOME` and `PATH`, optional `XDG_DATA_HOME` and
  `XDG_DATA_DIRS`.

This is contract policy, not evidence that those variables can yet be passed
safely to an executable on every platform.

## Response boundary

Because execution is unauthorized, the only valid schema-1 response is
`not_executed` with:

- matching request/module identity;
- no exit code, timeout, payload digest, stdout, or stderr;
- no writes or network attempts;
- `execution_not_authorized` as the bounded error code.

Tests reject fabricated success/output and unknown fields. Success/partial/
failure/timeout enum values are reserved for a later schema/gate and cannot
validate against the current preview plan.

## Remaining gate

There is no process spawn, helper executable, stdin/stdout transport, handle
inheritance policy, kill-on-timeout implementation, sandbox, capability broker,
receipt loader, or core/TUI/CLI integration. Before any child-process test, the
host needs a dedicated test helper and platform-specific proof for exact-path
open/execute race resistance, minimal environment behavior, working directory,
handle closure, output draining/truncation, timeout kill/reap, protocol framing,
and sandbox limitations.

A process boundary is not a sandbox. Production execution, dynamic libraries,
WASM, scripts, hooks, third-party code, elevated operations, and network access
remain blocked by [`module-trust-and-execution.md`](module-trust-and-execution.md).
