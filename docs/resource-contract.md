# Shared Resource and Efficiency Contract

`crates/resource-contract/` owns cross-cutting byte, record, timeout, and process
I/O ceilings. Modules consume these constants and typed limits instead of
quietly inventing larger or inconsistent budgets.

Schema-1 shared ceilings currently include:

- 64 MiB per verified artifact/action source/staging/package file;
- 64 KiB for small manifests, fixtures, desktop entries, receipts, and
  confirmation documents;
- 128 KiB per installed-registry document;
- 2 MiB per immutable transaction journal snapshot;
- 4 MiB per classified finding report, with 64 sources, 4,096 findings, and 16
  source references per finding;
- 16 MiB per complete inventory report;
- 1,024 installed-module records;
- 64 inventory sources, 512 PATH entries, 1,024 tool records, 4,096 software
  records, 4,096 service records, and 8,192 inventory events/warnings;
- 9,999 unique report-local redaction tokens, enough for the combined maximum
  PATH/tool/software/service path-bearing record set while preserving the
  fixed-width four-digit token grammar;
- 128 canonical diagnostic checks;
- 16 performance operations and 100 samples per operation;
- 64 KiB retained version-probe output;
- a 2-second version-probe timeout and 250-ms reader-close grace;
- module process limits of 10 seconds, 64 KiB stdin, 1 MiB stdout, and 64 KiB
  stderr;
- shared direct-process ceilings of 64 arguments × 512 bytes, 4 MiB retained per
  output stream, and 30 minutes maximum manager wall time.

`ProcessLimits` preserves the module protocol's schema-1 JSON shape while the
shared validator returns typed field violations. The protocol maps those to its
own contract messages. Artifact identity, action plans, module trust/staging,
package integrity, manifests, canonical registries, transaction snapshots,
inventory fixtures/collectors/probes, classified findings, privacy redaction, foundation diagnostics,
performance evidence, and module test framing now use the shared constants.

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
