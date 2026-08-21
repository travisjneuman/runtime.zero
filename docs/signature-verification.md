# Test-Key Detached Signature Verification

`crates/module-trust/` implements the second bounded module-trust stage: local,
detached Ed25519 verification with public test keys only. It is a library and
fixture contract with one read-only CLI review adapter; it is not a signer, key
store, installer, production trust root, or permission to execute a module.

## Contract

Schema version `1` uses two separately selected records:

- a signature envelope containing `scheme`, `key_id`, package ID/version, the
  exact manifest SHA-256 digest, and a detached signature;
- a caller-supplied trusted-test-key record containing the public key, matching
  key ID/scheme, explicit allowed package IDs, test-only purpose, and revocation
  flag.

The envelope never supplies its own trusted public key. Verification fails
closed for unknown fields, schemas or schemes; malformed IDs, versions, hashes,
keys, or signatures; key/envelope mismatches; unauthorized package IDs;
duplicate package authorizations; revoked keys; identity/digest drift; or an
invalid signature.

The current canonical UTF-8 message is:

```text
runtime.zero.package-signature.v1
scheme=<scheme>
key_id=<key-id>
package_id=<package-id>
package_version=<package-version>
manifest_sha256=<64 lowercase hex characters>
```

Every line ends with LF. Inputs are bounded and cannot contain control
characters through their field-specific validation. The signature therefore
binds the domain, scheme, selected key ID, package identity/version, and exact
manifest digest. A future package verifier must still verify that manifest's
explicit file hashes and immutable staged bytes; signature success alone does
not perform those steps.

The current local review surface is:

```bash
rz0 modules trust verify \
  --manifest path/to/rz0-module.json \
  --signature path/to/signature-envelope.json \
  --trusted-test-key path/to/trusted-test-key.json \
  --format json
```

The command reads bounded local files only. It validates the package manifest
and its declared file digests, computes the SHA-256 of the exact manifest bytes
used for validation, checks envelope identity/version/digest equality, and
calls the test-key verifier. A successful result still reports
`test_key_only: true`, `execution_authorized: false`, and
`writes_attempted: false`.

When the manifest sets `integrity.complete_file_set: true`, this review also
rejects undeclared regular files and bounded traversal hazards before reporting
the detached signature result. The signature envelope remains external to the
package directory in this contract.

## Cryptographic boundary

The contract uses `ed25519-dalek` 3.0 with strict Ed25519 verification and no
runtime signing API. Tests independently exercise RFC 8032 test vector 1 and a
canonical package-message signature. Committed fixtures contain only the RFC
public test key and detached signature. They are not production credentials or
an approved release key.

Ed25519 is the implemented local test scheme because it supports fixed-size
offline signatures and widely reviewable public verification. This does not
settle production key custody, hardware/offline storage, threshold policy,
rotation, recovery, release authorization, provenance, transparency, freshness,
or compromised-key response.

References:

- RFC 8032: https://www.rfc-editor.org/rfc/rfc8032.html
- `ed25519-dalek` 3.0 documentation: https://docs.rs/ed25519-dalek/3.0.0/ed25519_dalek/

## Immutable staging simulation

The same crate now validates schema-1 `module_staging_plan` fixtures. A valid
plan is simulation-only/dry-run/no-write, binds the transaction and publication
roots to package identity/version, requires exactly one manifest whose digest
matches the signed manifest, bounds 128 regular files to 64 MiB each and 512 MiB total, and
requires atomic publication plus preservation of failed unpublished stages.

Integration-test helpers bind that plan to a successful test-key verification,
read each source file once into bounded memory, verify size/digest, write those
same bytes with create-new semantics, verify the staged copy, and atomically
rename the staged directory inside one marked OS-temporary fixture root. Tamper,
symlink, identity/digest drift, existing destinations, and partial staging fail
closed. These helpers are not exported by the library or core.

See [`transaction-simulation.md`](transaction-simulation.md).

## Non-goals and next gate

There is intentionally no production key, private key, signing command,
manifest mutation, production signature policy, install/activation path,
production/core/inventory-module process launch, network fetch, release
workflow, or third-party trust decision. The CLI review adapter is not an
installer integration and cannot authorize lifecycle work.

A versioned first-party invocation/not-executed response protocol is
fixture-validated, and an explicit-feature lane executes only a Cargo-built test
helper to exercise bounded transport failures. Same-open-handle artifact
identity and non-authorizing Linux/Windows spawn leases now exist, while macOS
fails closed. The next bounded trust stage is to integrate exact identity with a
production contained host, close descriptor/handle races, and prove real
platform capability isolation. Before any developer artifact trial, the core
also needs production-grade package-file/signature routing, receipt binding,
platform isolation, and a dedicated review of production key/revocation policy. Release/distribution work remains a
separate explicit approval.
