# Local Release Packaging

The repository contains local, non-publishing build-runner helpers:

```bash
scripts/build-package.sh aarch64-apple-darwin /path/to/output
```

On macOS, an unsigned DMG can be assembled from the same canonical portable ZIP
contract:

```bash
scripts/build-dmg.sh aarch64-apple-darwin /path/to/output
```

```powershell
./scripts/build-package.ps1 -Target x86_64-pc-windows-msvc -OutputDirectory C:\output
```

The separate legacy Windows build-runner lane is:

```powershell
./scripts/build-legacy-windows.ps1 -Target x86_64-win7-windows-msvc -ProvisionPinnedToolchain
```

It pins `nightly-2026-07-29`, builds the Tier-3 standard library from `rust-src`,
and accepts only the Rust Windows-7-baseline x86/x86-64 targets. It still needs
a Windows MSVC/SDK linker and final-artifact tests on every legacy OS; it does
not make the ordinary artifact legacy-compatible.

They require a clean Git worktree, locked dependencies, a Rust toolchain, and a
linker capable of producing the selected target. Build runners use them; clean
compatibility hosts do not.

The wrapper builds `rz0` in release mode and calls
`scripts/package_release.py`. The packager:

- accepts the seven initial macOS/Windows/Linux x86-64/ARM64 targets plus the
  modern Windows x86 target;
- requires a full source commit and bounded version;
- rejects symlinked/non-regular or oversized binary/document inputs;
- embeds `rz0`, public policy files, target-filtered SPDX 2.3 JSON, deduplicated
  third-party license/notice evidence, and a strict artifact manifest;
- binds SBOM/notices size and SHA-256 into the artifact manifest;
- writes a deterministic, sorted ZIP with fixed metadata;
- emits a separate SHA-256 checksum;
- uses create-new publication and refuses occupied artifact paths;
- records the unsigned, non-notarized, non-Authenticode posture honestly;
- performs no upload, signing, account access, installation, deployment, or
  release creation.

`scripts/generate_release_metadata.py` traverses the Cargo metadata graph from
only the `runtime-zero` package, excludes dev-only edges, filters the selected
target, binds the exact final binary, and emits deterministic SPDX and notice
bytes. Registry checksums come from `Cargo.lock`; license/notice texts are read
only from direct package roots, bounded, hashed, and deduplicated. Missing text
remains explicit rather than being silently invented. This is evidence for
release/legal review, not legal advice.

The native Apple Silicon package has been independently generated twice with
identical ZIP, SBOM, and notices bytes, checksum-verified, extracted into a clean
temporary root, and exercised with `rz0 --version`, `rz0 doctor`, and the dry-run
scan. The Windows-7-baseline crates and custom standard library pass cross-target
workspace checks for x86 and x86-64; linked EXEs still require the Windows build
runner. Other targets still require link-capable build runners and artifact-only runtime hosts; `cargo check` is not
an executable artifact.

The DMG builder first creates and checksum-verifies the canonical portable ZIP,
then rejects missing/extra/duplicate/traversal/symlink/oversized entries before
preparing fixed-metadata content. SBOM and third-party notices are mandatory
members and their manifest digests are revalidated. The mounted image contains the original
artifact manifest plus `dmg-manifest.json`, which binds source ZIP, source commit,
target, binary content set, unsigned/notarized posture, and a deterministic
content digest. Unit tests cover valid preparation, checksum mismatch,
extra/traversal entries, and symlink entries.

Apple `hdiutil` emits variable filesystem/container metadata, so the DMG manifest
sets `container_reproducible: false`; the published per-build DMG SHA-256 remains
mandatory. This is an honest format limitation, not permission to skip content
reproducibility. The unsigned image will trigger Gatekeeper warnings and is not
created, uploaded, or installed automatically.

The portable ZIP and unsigned DMG are current artifact contracts. Installer,
DEB, RPM, and Arch package generation must consume the same final binary/
manifest evidence and add format-specific install, upgrade, rollback, and
uninstall tests rather than creating independent trust logic.

See [`free-release-distribution.md`](free-release-distribution.md) and
[`support-policy.md`](support-policy.md).
