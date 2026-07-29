# Module protocol fixtures

These synthetic JSON records model a read-only inventory invocation preview and
the required `not_executed` response. They contain no executable bytes,
environment values, credentials, private paths, process output, or execution
authorization.

The explicit-feature process tests construct a separate outer test-only request
at runtime and copy the Cargo-built `rz0-protocol-test-child` into a guarded
OS-temp root. No executable fixture, absolute path, or environment value is
committed here.

`blocked-production-execution.json` is the canonical schema-1 production gate
assessment. It lists every required artifact, capability, executable identity,
process, runtime, and transaction gate while keeping product execution
unconditionally blocked. Test-only transport evidence is intentionally not
promoted to production proof.
