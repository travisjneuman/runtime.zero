# Test-Key Detached Signature Verification

`crates/module-trust/` implements the second bounded module-trust stage: local,
detached Ed25519 verification with public test keys only. It is a library and
fixture contract, not a signer, key store, installer, CLI command, production
trust root, or permission to execute a module.

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

## Non-goals and next gate

There is intentionally no production key, private key, signing command,
manifest mutation, signature-file loader, core integration, install/activation
path, module process launch, network fetch, release workflow, or third-party
trust decision.

The next bounded trust stage is immutable staging and transaction simulation
under temporary runtime.zero-owned fixture roots. Before any developer artifact
trial, the core also needs exact package-file/signature routing, capability
grant enforcement, receipt binding, process protocol isolation, and a dedicated
review of production key/revocation policy. Release/distribution work remains a
separate explicit approval.
