# Opened Artifact Identity Primitive

`crates/artifact-identity/` is a foundation library for opening one
receipt-relative artifact, hashing the bytes from that open handle, recording
platform file identity, revalidating the path, and returning the same rewound
handle. It performs no execution, installation, staging, trust decision, or
system mutation.

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
hardlink rejection, and that the returned Unix handle continues to expose the
original verified bytes after the path is replaced. Windows and Linux target
checks are compile evidence until real runtime tests exist.

## Security boundary

This primitive improves validation-to-use behavior because callers can consume
the already verified open file instead of reopening an untrusted path. It does
**not** by itself close executable validation-to-spawn races:

- Unix root-relative no-follow opening now anchors component traversal to held
  directory handles, but macOS and the current standard process APIs still
  launch executables by path;
- Windows now denies write/delete sharing on the held final handle and records
  its File ID, but still needs root-handle-relative traversal, an approved
  CreateProcess binding strategy, and real reparse/replace/share-conflict tests;
- Linux handle-based execution needs a separately reviewed `fexecve`/equivalent
  host and runtime proof;
- same-user filesystem authority and platform code-signing/sandbox policy remain
  relevant.

Therefore this crate is evidence for the `executable_identity_pinned` and
`executable_replacement_race_closed` production gates, not proof that those gates
are complete. No production execution assessment should mark either gate proven
until the verified handle is bound to the actual platform execution primitive
and adversarial replacement tests pass.

See [`module-process-protocol.md`](module-process-protocol.md),
[`module-trust-and-execution.md`](module-trust-and-execution.md), and
[`production-readiness.md`](production-readiness.md).
