# Foundation Error Contract

`crates/error-contract/` owns stable machine-readable error codes and their
security/retry semantics. Modules must not create incompatible retry, privacy,
or policy meanings for shared failures.

Schema-1 codes cover authorization, contract validation, unsupported platform or
operation, capabilities, trust, artifact identity, input/output limits, timeout,
cancellation, conflicts, transactions/recovery, permission, resource pressure,
I/O availability, and internal invariants.

Each code classifies:

- its foundation category;
- whether correction requires changed input, a changed environment, manual
  recovery, or must never be retried;
- whether automatic retry is allowed;
- whether accompanying detail must be redacted;
- whether the code itself is safe for JSON.

Schema 1 allows no automatic retries. Every detail is redacted by default; a
stable code is not permission to expose paths, command output, registry keys,
usernames, hostnames, tokens, or arbitrary OS error text. Unknown serialized
codes fail deserialization.

The module protocol now uses the typed code for its unchanged JSON value
`execution_not_authorized`, replacing free-form error-code acceptance. Human
messages remain separate presentation data and should not be parsed for policy.

This crate does not log, retry, display, localize, redact, or recover anything.
`crates/privacy-contract/` supplies the bounded report-local redaction primitive;
it does not make arbitrary OS error text safe automatically. Future
CLI/TUI/module/transaction adapters should map internal failures at their
boundary, preserve a causal chain privately where safe, and emit only bounded
privacy-reviewed detail. Adding a code requires a compatibility and threat-model
review; changing the meaning of an existing code requires a new schema.

See [`architecture.md`](architecture.md),
[`module-process-protocol.md`](module-process-protocol.md), and
[`production-readiness.md`](production-readiness.md).
