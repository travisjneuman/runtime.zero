# Foundation Diagnostics Contract

`crates/diagnostics-contract/` owns the bounded, deterministic, privacy-safe
`foundation_diagnostics` schema used by `rz0 doctor`.

Schema 1 is read-only, reports `writes_attempted: false`, and cannot authorize
production execution. It binds the exact canonical configuration SHA-256 and
contains exactly nine ordered checks for runtime identity, platform identity,
configuration policy, safety posture, store mutation policy, module execution,
network policy, external automation, and privacy defaults. Passing checks cannot
carry error codes; blocked or unavailable checks require a shared typed
foundation error. Summaries must exactly match the check list, and the foundation
ceiling is 128 checks.

Hostnames, user identities, current directories, environment values, and raw
paths are omitted. This replaces the earlier text-only doctor output that
included the current working directory.

```text
rz0 doctor
rz0 doctor --format json
```

Both renderings come from the same validated report. Diagnostics describe
posture and availability; they do not grant capability, execution, mutation,
retry, recovery, or release authority.
