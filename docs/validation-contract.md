# Validation Contract

`crates/validation-contract/` is the allocation-free lexical policy layer for
untrusted contract inputs. Foundation and module-facing parsers use it instead
of independently accepting subtly different identifiers, hashes, versions, or
paths.

It centralizes:

- bounded lowercase dotted IDs and ledger IDs;
- module IDs and bounded ASCII versions;
- canonical lowercase SHA-256 and fixed-length hexadecimal values;
- normalized platform-neutral relative paths;
- bounded ASCII detail and evidence references;
- cross-platform absolute local path recognition.

Relative paths reject drive prefixes, URI schemes, backslashes, control bytes,
empty components, `.` components, and `..` traversal before platform path APIs
see them. SHA-256 text is lowercase-only so equal digests have one serialized
form. Validation is allocation-free and adds no external dependency.

The crate owns lexical policy, not semantic authorization. Callers still enforce
schema-specific ceilings, reserved namespaces, required path classes, exact
capabilities, trust, and write authority. A lexically valid path is never by
itself permission to read or write it.
