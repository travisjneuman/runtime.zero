# Opened-Directory Filesystem Foundation

`crates/secure-fs/` centralizes filesystem operations that must be relative to an
already opened directory instead of repeatedly resolving mutable absolute paths.
It is used by Unix store initialization and transaction journal persistence.

## Unix implementation

The Unix implementation uses `open`/`openat`, `mkdirat`, `linkat`, and `unlinkat`
with close-on-exec and no-follow flags. Operations accept exactly one normal
child component and provide:

- direct directory opens that reject symlinks;
- private `0700` child-directory creation;
- private `0600` create-new child files with bounded bytes and file/directory
  synchronization;
- no-follow, single-hardlink regular-file opens and bounded reads;
- root-relative lock-file opens;
- no-replace publication by atomically linking a complete pending file into its
  final held directory, synchronizing, then retiring the pending link;
- typed errors mapped into the shared foundation error vocabulary.

Holding the directory descriptor prevents a path replacement from redirecting a
write: adversarial tests rename the visible root, replace it with another
directory, and verify that the operation still reaches only the originally
opened directory.

If interruption occurs after final-link publication but before pending-link
retirement, the operation reports recovery-required and the two-link state fails
normal single-link validation. It is never silently accepted or overwritten.

## Windows boundary

Windows can open and inspect a direct directory handle with reparse-point
rejection, but schema 1 deliberately returns `unsupported_operation` for child
mutation. Path-based emulation would not provide equivalent root-handle race
closure. A reviewed Windows implementation requires NT root-relative handle
semantics, reparse/File-ID tests, ACL policy, atomic publication, directory-
metadata flush evidence, and runtime proof from Windows 7 through current
client/server targets.

This explicit unsupported result keeps Windows an equal release blocker rather
than weakening its security contract. Windows-target compilation is build
evidence only.

## Authority boundary

A `SecureDirectory` handle conveys location, not policy authority. It does not
grant capabilities, validate an action plan, confirm a mutation, establish
trust, execute code, or authorize deletion. Modules must consume higher-level
foundation transaction APIs rather than calling filesystem primitives directly.

See [`transaction-journal.md`](transaction-journal.md),
[`store-and-routing-contract.md`](store-and-routing-contract.md), and
[`production-readiness.md`](production-readiness.md).
