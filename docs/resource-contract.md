# Shared Resource and Efficiency Contract

`crates/resource-contract/` owns cross-cutting byte, record, timeout, and process
I/O ceilings. Modules consume these constants and typed limits instead of
quietly inventing larger or inconsistent budgets.

Schema-1 shared ceilings currently include:

- 64 MiB per verified artifact/action source/staging/package file;
- 64 KiB for small manifests, fixtures, desktop entries, and test frames;
- 512 PATH entries per source;
- 4,096 application records per bounded collector;
- 64 KiB retained version-probe output;
- a 2-second version-probe timeout and 250-ms reader-close grace;
- module process limits of 10 seconds, 64 KiB stdin, 1 MiB stdout, and 64 KiB
  stderr.

`ProcessLimits` preserves the module protocol's schema-1 JSON shape while the
shared validator returns typed field violations. The protocol maps those to its
own contract messages. Artifact identity, action plans, module trust/staging,
package integrity, manifests, inventory fixtures/collectors/probes, and module
test framing now use the shared constants.

These are safety ceilings, not performance targets or permission to allocate the
maximum eagerly. Implementations should stream, short-circuit, preallocate only
from already bounded evidence, and retain useful partial results. Any increase
requires measured fixtures, memory/disk/output impact, abuse analysis, and a
schema/compatibility review. Modules may impose a smaller domain-specific limit
but may not exceed a foundation ceiling.

The contract performs no I/O, allocation beyond a four-entry violation vector,
process execution, logging, retry, or mutation. Measured startup, scan, memory,
TUI, transaction, and recovery budgets remain a release workstream.

See [`architecture.md`](architecture.md),
[`module-process-protocol.md`](module-process-protocol.md), and
[`production-readiness.md`](production-readiness.md).
