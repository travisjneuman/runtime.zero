# Module Process Protocol Preview

`crates/module-protocol/` defines a versioned host/module process contract while
keeping product execution blocked. Schema version `1` authorizes nothing: a
valid module plan is read-only, dry-run, offline, path-redacted, and explicitly
sets both `execution_authorized` and `execution_attempted` to `false`.

An explicit Cargo feature now enables a separate integration-test transport and
Cargo-built helper. That outer test contract authorizes only
`rz0-protocol-test-child`; it does not authorize `rz0-inventory`, a staged module,
or any caller-selected executable. The core, CLI, TUI, and default protocol
build do not depend on or invoke the helper.

## Invocation plan

The schema binds:

- request ID and exact `first-party.inventory` identity/version;
- target platform and the single `collect_inventory` operation;
- a receipt-relative `bin/` executable path, lowercase SHA-256, and bounded
  exact size;
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

Every plan uses the shared foundation vocabulary but schema 1 accepts only its
read-only protocol subset. Process-environment and filesystem-metadata reads are
required; action/network/write/elevation capabilities are rejected. Platform
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

This remains contract policy, not evidence that environment values are safe for
an arbitrary module on every platform.

## Product response boundary

Because module execution is unauthorized, the only valid schema-1 module
response is `not_executed` with:

- matching request/module identity;
- no exit code, timeout, payload digest, stdout, or stderr;
- no writes or network attempts;
- the shared typed foundation error code serialized as
  `execution_not_authorized`; free-form error codes are rejected.

Tests reject fabricated success/output and unknown fields. Success/partial/
failure/timeout enum values remain reserved for a later schema/gate and cannot
validate against the current module preview plan.

## Production execution assessment

Schema-1 `production_execution_assessment` is a separate fail-closed foundation
contract. It requires the exact canonical set of 29 artifact, confirmation,
capability, executable-identity, process, runtime, and transaction gates for one
module/platform assessment. Proven gates require bounded mechanism and evidence
identifiers; missing/unsupported gates cannot masquerade as proof. Gates must be
unique and canonically ordered.

The assessment's only decision value is `blocked`, and
`product_execution_authorized` must be `false`. Even an assessment containing
synthetic proof for every gate cannot authorize product execution under schema
1. This keeps future readiness accounting separate from the no-execution module
protocol and prevents test-helper evidence from becoming production authority.
See [`production-readiness.md`](production-readiness.md).

## Developer-trial invocation boundary

The foundation now has one explicit outer developer-trial lane for the
promoted `first-party.inventory` package:

```text
rz0 modules invoke --developer-trial --dry-run --module-id first-party.inventory --store-root PATH
rz0 modules invoke --developer-trial --apply --module-id first-party.inventory --store-root PATH --challenge-issued-unix-seconds SECONDS --confirm PHRASE
```

The dry-run requires a valid promoted `installed_inactive` record, a
test-key-only `receipts/install-*.json` receipt, a supported installed manifest,
complete package integrity, and a declared `bin/rz0-inventory` executable. The
apply lane reopens and rehashes that exact executable, holds its identity through
the shared Rust process host, clears the parent environment, passes no shell or
user arguments, enforces the bounded timeout/output contract, and accepts only a
path-redacted `inventory_report` with `read_only=true` and
`writes_attempted=false`.

This is deliberately not schema-1 production execution authorization. It does
not activate the registry, write a lifecycle receipt, grant third-party trust,
or enforce filesystem/network/privilege/syscall sandboxing. The report marks
`developer_trial=true` and `product_execution_authorized=false`; absence of a
complete immutable package or any response drift fails closed.

## Explicit-feature test-child transport

`cargo test -p rz0-module-protocol --all-features` enables a private test lane.
The test setup copies the Cargo-built helper into a marked, prefixed direct child
of the canonical OS temp root and constructs a receipt-like `bin/` path. Before
spawn, the test host requires:

- an outer `test_only` contract with explicit test-helper authorization;
- a still-unauthorized valid schema-1 module preview nested inside it;
- the exact helper identity, copied path, regular-file shape, size, and SHA-256,
  verified through the shared opened-artifact identity primitive;
- no symlink in the receipt-relative executable path;
- an exact environment-name/value map matching the preview allowlist;
- a marked direct working directory inside the isolated test root;
- on Unix, no observed non-standard descriptor whose `FD_CLOEXEC` bit is clear.

The test host invokes the helper directly with no shell, PATH search, or
arguments. Linux and Windows builds consume the borrow-scoped verified-
executable binding through spawn and retain it until child creation; guarded
macOS tests still use the canonical copied test path because production macOS
binding deliberately fails closed. It clears the parent environment, sets only the
explicit map, pipes only stdin/stdout/stderr, sends one bounded JSON request,
and requires one strict JSON response. Stdout and stderr are drained
concurrently while memory retention stays bounded. Tests cover:

- successful framing, exact environment names, zero arguments, and working-dir
  marker proof;
- malformed output and nonzero exit rejection;
- a stderr burst large enough to exercise concurrent draining;
- stdout/stderr flooding with continued draining and fail-closed byte ceilings;
- allocation-minimal shared cancellation/deadline polling followed by direct-
  child kill and reap;
- process-tree timeout teardown that terminates a helper-spawned sleeping
  descendant and closes its inherited pipes: a fresh process group on Unix and
  a private kill-on-close Job Object with a bounded active-process count on
  Windows;
- a deliberately inheritable Unix descriptor rejected before spawn;
- authorization, identity, digest, environment, and Unix symlink drift before
  spawn;
- response rejection if the helper claims a write;
- post-use digest, size, link-count, held identity, and current-path identity
  revalidation through the original artifact handle.

Cleanup revalidates the canonical temp parent, root prefix, regular marker, and
exact marker content before removing the isolated test root. The helper source
contains no filesystem mutation beyond reading the working marker and no
network operation. This is test evidence only; it is not an execution API.

## Unresolved isolation gates

The test lane deliberately does **not** claim production isolation:

- Linux and Windows test-host builds bind the held verified artifact through
  spawn, but this still lacks real Windows runtime proof and broader capability
  enforcement;
  guarded macOS tests spawn only their copied test helper by canonical path
  while production macOS binding remains unsupported;
- the standard process boundary does not enforce filesystem, registry, process,
  network, or syscall capabilities;
- shared Unix preflight enumeration rejects currently observed descriptors with
  `FD_CLOEXEC` clear, but a descriptor created after that audit remains a race;
  Windows production launch uses an explicit inherited-handle list; general
  module execution still has no capability broker;
- Unix tests assign the helper to a fresh process group and kill the group on
  timeout, but the host reaps only its direct child; the shared cancellation
  token is not yet a production host or platform teardown implementation;
- Windows-target test support retains a private kill-on-close Job Object helper
  for fixture coverage; the updater process host now assigns its production Job
  Object before behavior dispatch, but this is not a sandbox or real Windows
  runtime proof;
- descendants that escape the assigned group or retain pipes through another
  process could still delay reader completion;
- Windows reparse/File ID, macOS sandbox/code-signing, and Linux namespace/
  seccomp/landlock behavior have no runtime proof;
- there is no production receipt loader, capability broker, journal, installed
  module path, or core/TUI/CLI integration.

A process boundary is not a sandbox. Production module execution, dynamic
libraries, WASM, scripts, hooks, third-party code, elevated operations, and
network access remain blocked by
[`module-trust-and-execution.md`](module-trust-and-execution.md).
