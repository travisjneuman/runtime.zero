# Final-Artifact Performance Contract

`crates/performance-contract/` owns bounded, machine-validatable command
performance evidence. Measurements never authorize release.

Schema 1 requires 10–100 successful samples for exactly six ordered final-
artifact operations:

1. version;
2. text diagnostics;
3. JSON diagnostics;
4. text core dry-run scan;
5. JSON core dry-run scan;
6. JSON dashboard.

The initial cross-target baseline requires p95 wall time at or below one second,
maximum wall time at or below two seconds, maximum resident memory at or below
64 MiB, and combined output at or below 2 MiB for every operation. These are
release-blocking ceilings, not optimization goals; target-specific acceptance
may narrow them but cannot expand them silently.

Evidence binds target, source commit, final artifact SHA-256, sample count,
percentiles, maximum time/RSS/output, and pass/blocked decision. Percentiles must
be ordered, every operation must have the exact successful sample count, unknown
fields fail, and `release_authorized` must be false.

`scripts/benchmark_final_artifact.py` measures an already built single-link
binary on POSIX hosts with a minimal deterministic environment. It can select an
ARM64 or x86-64 slice of a macOS universal binary. It performs only read-only
commands, writes one create-new evidence document, and returns nonzero when a
budget is exceeded. Host/OS/runtime context still belongs in the private release
ledger; one fast machine cannot prove another target.

The paused macOS universal artifact from product commit `53d1e3d` passed
25-sample ARM64 and Rosetta x86-64 runs for all six operations. Worst ARM64 p95
was 19.286 ms, maximum was 20.269 ms, and peak observed RSS was 5,062,656
bytes. Worst Rosetta p95 was 35.238 ms, maximum was 36.070 ms, and peak observed
RSS was 6,344,704 bytes. These are local process-launch measurements, not older
macOS, Intel hardware, terminal, sustained-load, or cross-platform proof.

Schema 1 does not directly time `rz0 apps` or interactive TUI startup. Core scan
uses the same live collector and final-artifact PTY smoke exercises TUI startup,
but a future contract revision should add explicit catalog/TUI operations rather
than silently changing schema 1's exact six-operation set.
