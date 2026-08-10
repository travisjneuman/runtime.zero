# Process Host Foundation

`crates/process-host/` centralizes process I/O and containment primitives so
modules cannot invent private subprocess stacks.

The default library provides:

- a bounded direct process transport shared by explicit manager mutations and
  read-only probes; this crate itself grants no mutation authority;
- bounded output capture that continues draining to EOF to prevent pipe
  deadlock, retains only the caller ceiling, and uses saturating byte accounting;
- Unix enumeration of non-standard descriptors and fail-closed rejection of any
  descriptor without `FD_CLOEXEC`;
- explicit failure on Windows because a complete inherited-handle audit is not
  yet implemented;
- Unix pre-exec dedicated process-group setup and whole-group termination/reap.
  This contains ordinary descendants but is not a sandbox and cannot prevent a
  hostile child from creating a new session;
- caller-supplied first-reason cancellation plus an atomic monotonic deadline,
  with typed cancellation/timeout distinction and no retry;
- a process-wide serialized inheritable-descriptor audit/spawn boundary for
  mutating Unix launches;
- an explicit process transport with absolute direct-executable and
  working-directory checks, an explicit bounded environment allowlist, null
  stdin, bounded concurrent stdout/stderr drains, and a monotonic timeout. The
  same primitive serves probes and manager apply actions; callers still need
  exact plan, capability, confirmation, transaction, and platform gates.

The opt-in inventory version-probe adapter and updater manager apply lane consume
the shared drain, descriptor audit, Unix group teardown, process ceilings, and
atomic deadline/cancellation signal. The mutating lane accepts a borrow-scoped
`BoundExecutable`: Linux native-ELF manager execution substitutes the held `/proc/self/fd`
identity, keeps the
lease through spawn, and revalidates after child creation. It clears the
environment, uses `/` as working directory, rejects truncated streams, and reaps
on timeout or cancellation. Windows probes/apply fail closed at production
handle/containment policy rather than using the post-spawn test Job assignment.
Hostile session escape and OS capability isolation remain production gates, so
this adapter does not authorize general module execution.

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
