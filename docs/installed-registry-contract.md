# Installed Registry Contract

`crates/registry-contract/` owns the installed-module registry model, validation,
canonical serialization, and digest policy. Core reporting and transaction
publication consume this crate; modules do not define private registry formats.

Schema 1 requires:

- `schema_version: 1` and no unknown fields;
- at most 1,024 module records and at most 128 KiB per document;
- unique module IDs in ascending canonical order;
- valid non-`core.*` module IDs and bounded versions;
- exact `modules/<id>/<version>/rz0-module.json` manifest paths;
- normalized `receipts/*.json` receipt paths;
- when present, exact `modules/<id>/<version>` module directories;
- forward-slash relative paths with no traversal, drive, URI, control, or empty
  components.

Canonical output is compact struct-order JSON followed by one newline. Registry
state digests bind those canonical bytes. A pre-existing registry is separately
bound by the SHA-256 of the exact validated bytes that were confirmed, so a
format or content change before commit is a conflict.

The contract validates evidence only. It does not install, activate, execute,
remove, trust, or authorize a module. Atomic publication belongs to the shared
transaction coordinator described in
[`transaction-journal.md`](transaction-journal.md).
