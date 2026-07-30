# Process Host Foundation

`crates/process-host/` centralizes process I/O and containment primitives so
modules cannot invent private subprocess stacks.

The default library provides:

- bounded output capture that continues draining to EOF to prevent pipe
  deadlock, retains only the caller ceiling, and uses saturating byte accounting;
- Unix enumeration of non-standard descriptors and fail-closed rejection of any
  descriptor without `FD_CLOEXEC`;
- explicit failure on Windows because a complete inherited-handle audit is not
  yet implemented.

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

This crate exposes no production module runner. Cancellation, trust,
capabilities, signatures, sandboxing, network denial, executable identity,
confirmation, and transactions remain independent mandatory gates.
