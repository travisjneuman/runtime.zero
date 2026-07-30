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
- atomic replacement for transaction-coordinated registry publication;
- opened-file exclusive nonblocking locks;
- explicit effective-user ownership and `0700`/`0600` privacy verification;
- typed errors mapped into the shared foundation error vocabulary.

Holding the directory descriptor prevents a path replacement from redirecting a
write: adversarial tests rename the visible root, replace it with another
directory, and verify that the operation still reaches only the originally
opened directory.

If interruption occurs after final-link publication but before pending-link
retirement, the operation reports recovery-required and the two-link state fails
normal single-link validation. It is never silently accepted or overwritten.

## Windows boundary

The compile-checked Windows implementation uses `NtCreateFile` with a held
`RootDirectory`, `FILE_OPEN_REPARSE_POINT`, synchronous handles, exact one-child
UTF-16 names, and create/open dispositions. `NtSetInformationFile` provides
no-replace or replace-enabled root-relative atomic rename and root-relative
unlink. File IDs/link counts, reparse attributes, directory handles, and
`LockFileEx` locking are checked without path-based mutation emulation.

Windows owner/DACL inspection is now compile-checked. It queries the process
user SID and handle security descriptor, requires exact user ownership and a
non-null bounded DACL, and accepts allow ACEs only for that user, LocalSystem, or
Builtin Administrators. At least one user allow ACE is required; unsupported ACE
types and grants to any other principal fail closed. Inherited ACEs receive the
same principal policy rather than being trusted because they were inherited.

This remains build evidence, not runtime proof. `store init --yes` stays
structurally blocked on Windows because safe initial ACL creation and the full
runtime filesystem matrix are not proven. Release support still requires real
owner/DACL and inherited-ACL tests, token/elevation cases, reparse/File-ID
adversarial tests, atomicity and directory-flush evidence, and final-artifact
runtime proof from Windows 7 through current client/server targets.

## Authority boundary

A `SecureDirectory` handle conveys location, not policy authority. It does not
grant capabilities, validate an action plan, confirm a mutation, establish
trust, execute code, or authorize deletion. Modules must consume higher-level
foundation transaction APIs rather than calling filesystem primitives directly.

See [`transaction-journal.md`](transaction-journal.md),
[`store-and-routing-contract.md`](store-and-routing-contract.md), and
[`production-readiness.md`](production-readiness.md).
