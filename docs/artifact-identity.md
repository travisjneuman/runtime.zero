# Opened Artifact Identity Primitive

`crates/artifact-identity/` is a foundation library for opening one
receipt-relative artifact, hashing the bytes from that open handle, recording
platform file identity, revalidating the path, returning the same rewound
handle, and deriving a platform-specific identity-bound spawn lease where a
reviewed primitive exists. `revalidate_verified_artifact` can rehash that held handle after use,
recheck identity/link count/size and current-path identity, then rewind it on
success. The crate performs no execution, installation, staging, trust decision,
or system mutation.

## Contract

`open_verified_artifact(root, relative_path, expectation)` requires:

- an existing direct non-symlink/non-reparse directory root;
- a normalized relative path with no absolute, traversal, URL-like, backslash,
  empty, dot, or control-character components;
- direct directory components and a final regular file;
- an expected lowercase SHA-256 and size no greater than 64 MiB;
- a platform identity that can be queried from the opened handle;
- exactly one filesystem link, rejecting hardlinked receipt artifacts.

The function:

1. rejects unsafe root/path filesystem types;
2. on Unix, holds the root directory and walks every component with
   root-relative `openat`, `O_NOFOLLOW`, `O_CLOEXEC`, and `O_DIRECTORY` for
   intermediate components;
3. opens the final file read-only without following a symlink;
4. reads metadata and identity from that live handle;
5. reads at most 64 MiB from the same handle;
6. verifies metadata size, actual byte count, expected size, and SHA-256;
7. canonicalizes and reopens the current path;
8. requires the current path identity to match the original opened identity;
9. rewinds and returns the original verified handle.

Unix identity uses device, inode, and link count. Windows opens the final path
with `FILE_FLAG_OPEN_REPARSE_POINT` and `FILE_SHARE_READ` only, then uses
`GetFileInformationByHandle` volume serial number, file index, and link count;
it does not rely on unstable standard-library by-handle metadata methods. This
Windows behavior is target-compiled but still needs adversarial runtime proof.

Native tests prove digest/size/path rejection, symlinked root/artifact rejection,
hardlink rejection, and that post-use revalidation detects a Unix path
replacement while the returned handle continues to expose the original bytes.
The native macOS binding test deliberately proves fail-closed unsupported
behavior. Windows and Linux binding target checks remain compile evidence until
real runtime tests exist.

## Security boundary

`bind_verified_executable` returns a borrow-scoped `BoundExecutable` and always
reports `execution_authorized: false`:

- Linux/Android use `/proc/self/fd/<held-fd>` and verify its device/inode against
  the opened artifact before exposing the launch path;
- Windows exposes the canonical path only while retaining the original
  share-read-only handle, which denies normal write/delete replacement across
  `CreateProcess` path resolution;
- macOS and other Unix systems fail closed because `/dev/fd` is not a reliable
  executable primitive and no reviewed handle-to-spawn implementation exists.

Linux and Windows still require adversarial runtime tests integrated with the
actual contained process host. Same-user filesystem authority, descriptor/handle
inheritance, platform code-signing, and sandbox policy remain relevant.

Therefore this crate is evidence for the `executable_identity_pinned` and
`executable_replacement_race_closed` production gates, not proof that those gates
are complete. No production execution assessment should mark either gate proven
until the verified handle is bound to the actual platform execution primitive
and adversarial replacement tests pass.

See [`module-process-protocol.md`](module-process-protocol.md),
[`module-trust-and-execution.md`](module-trust-and-execution.md), and
[`production-readiness.md`](production-readiness.md).
