# Process Host Foundation

`crates/process-host/` centralizes process I/O and containment primitives so
modules cannot invent private subprocess stacks.

The default library provides:

- bounded output capture that continues draining to EOF to prevent pipe
  deadlock, retains only the caller ceiling, and uses saturating byte accounting;
- Unix enumeration of non-standard descriptors and fail-closed rejection of any
  descriptor without `FD_CLOEXEC`;
- explicit failure on Windows because a complete inherited-handle audit is not
  yet implemented;
- Unix pre-exec dedicated process-group setup and whole-group `SIGKILL` teardown.
  This contains ordinary descendants but is not a sandbox and cannot prevent a
  hostile child from creating a new session;
- an explicit read-only probe transport with absolute direct-executable and
  working-directory checks, cleared environment, null stdin, bounded concurrent
  stdout/stderr drains, and a monotonic timeout. The transport is a primitive,
  not manager/module authority, and still requires artifact identity, trust,
  capabilities, and platform proof before product use.

The opt-in inventory version-probe adapter now consumes the shared drain,
descriptor audit, Unix group teardown, process ceilings, and atomic deadline
signal. It clears the environment, uses `/` as working directory, rejects
truncated streams, and reaps on timeout. Windows probes fail closed at handle/
containment policy rather than using the post-spawn test Job assignment. Exact
executable trust/identity-to-spawn and hostile session-escape remain production
gates, so this adapter does not authorize general module execution.

The `test-support` feature contains guarded helper-only process groups and Job
Objects. Unix helpers enter a fresh process group and timeout teardown signals
the group. Windows build evidence uses a private kill-on-close Job Object with a
two-process ceiling, but assignment occurs after creation and is therefore not a
race-free production mechanism.

The module-protocol test transport now consumes this crate rather than owning
capture and containment code. On Linux and Windows builds it also creates the
child from the borrow-scoped verified executable binding and drops that lease
only after spawn. macOS guarded tests retain their explicit test-helper path;
production macOS execution remains blocked because no reviewed exact handle-to-
spawn primitive exists.

This crate exposes no production module runner. The narrow Unix group primitive
does not create execution authority. Trust,
capabilities, signatures, sandboxing, network denial, executable identity,
confirmation, and transactions remain independent mandatory gates.
