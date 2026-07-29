# Local Release Packaging

The repository contains local, non-publishing build-runner helpers:

```bash
scripts/build-package.sh aarch64-apple-darwin /path/to/output
```

```powershell
./scripts/build-package.ps1 -Target x86_64-pc-windows-msvc -OutputDirectory C:\output
```

They require a clean Git worktree, locked dependencies, a Rust toolchain, and a
linker capable of producing the selected target. Build runners use them; clean
compatibility hosts do not.

The wrapper builds `rz0` in release mode and calls
`scripts/package_release.py`. The packager:

- accepts only the six initial macOS/Windows/Linux x86-64/ARM64 Rust targets;
- requires a full source commit and bounded version;
- rejects symlinked/non-regular or oversized binary/document inputs;
- embeds only `rz0`, README, license, safety/security policy, and a strict
  artifact manifest;
- writes a deterministic, sorted ZIP with fixed metadata;
- emits a separate SHA-256 checksum;
- uses create-new publication and refuses occupied artifact paths;
- records the unsigned, non-notarized, non-Authenticode posture honestly;
- performs no upload, signing, account access, installation, deployment, or
  release creation.

The current native Apple Silicon package was independently generated twice with
identical ZIP bytes, checksum-verified, extracted into a clean temporary root,
and exercised with `rz0 --version` and `rz0 doctor`. Other targets still require
link-capable build runners and artifact-only runtime hosts; `cargo check` is not
an executable artifact.

The portable ZIP is the first artifact contract. DMG, installer, DEB, RPM, and
Arch package generation must consume the same final binary/manifest evidence and
add format-specific install, upgrade, rollback, and uninstall tests rather than
creating independent trust logic.

See [`free-release-distribution.md`](free-release-distribution.md) and
[`support-policy.md`](support-policy.md).
