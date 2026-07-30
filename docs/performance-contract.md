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

The current macOS universal artifact passed 25-sample ARM64 and Rosetta x86-64
runs for all six operations. ARM64 p95 values were below 9.5 ms with peak
observed RSS at 2.0 MiB; Rosetta p95 values were below 19.3 ms with peak observed
RSS below 3.5 MiB. These are local process-launch measurements, not older macOS,
Intel hardware, terminal, sustained-load, or cross-platform proof.
