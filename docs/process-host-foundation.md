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
- Windows `CreateProcessW` launch with a private kill-on-close Job Object
  assigned before first instruction and an explicit inherited-handle list;
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
`BoundExecutable`: Linux native-ELF manager execution substitutes the held
`/proc/self/fd` identity, keeps the lease through spawn, and revalidates after
child creation; macOS revalidates the direct path's device/inode/link/size/digest
immediately before spawn. It clears the
environment, uses `/` as working directory, rejects truncated streams, and reaps
on timeout or cancellation. Windows probes/apply use the same production host;
the verified executable's deny-write/delete lease remains held through
CreateProcessW, and only the three intended standard handles are inherited.
Hostile descendants, reparse/ACL guarantees, and OS capability isolation remain
production gates, so this adapter does not authorize general module execution.

The `test-support` feature contains guarded helper-only process groups and Job
Objects. Unix helpers enter a fresh process group and timeout teardown signals
the group. Windows helpers retain their small post-spawn assignment fixture for
legacy transport tests; production code uses the pre-start attribute-list Job
Object path above.

The module-protocol test transport now consumes this crate rather than owning
capture and containment code. On Linux and Windows builds it also creates the
child from the borrow-scoped verified executable binding and drops that lease
only after spawn. macOS guarded tests cover the path revalidation binding;
Darwin's lack of a public fexecve-style primitive means this remains a
last-moment path binding rather than a held-descriptor substitution.

This crate exposes no production module runner. The narrow Unix group primitive
does not create execution authority. Trust,
capabilities, signatures, sandboxing, network denial, executable identity,
confirmation, and transactions remain independent mandatory gates.
