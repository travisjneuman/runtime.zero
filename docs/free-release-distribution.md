# Free GitHub Release and Signing Policy

`runtime.zero` will not require a paid Apple Developer Program membership,
paid Windows code-signing certificate, paid package repository, or paid release
service for its initial public distribution. GitHub-hosted source and release
artifacts remain the intended no-cost channel.

## Consequences of the no-paid-signing decision

- macOS archives or DMGs cannot carry a trusted Developer ID signature or Apple
  notarization without the corresponding Apple account. Ad-hoc signing can help
  local integrity but does not establish publisher identity; Gatekeeper may warn
  or block first launch.
- Windows EXEs/installers without an Authenticode certificate may trigger
  SmartScreen or unknown-publisher warnings.
- Linux archives can be verified independently, while native repository trust
  depends on each repository/channel's signing and publication process.

The project must describe these limits plainly. It must never claim “signed” or
“notarized” when an artifact has only a checksum, ad-hoc signature, test key, or
keyless transparency record.

## Free assurance stack

Every release candidate should produce, locally before publication:

1. deterministic versioned archives/packages per target;
2. SHA-256 checksums generated from final bytes;
3. an SPDX or CycloneDX SBOM;
4. license/notices inventory;
5. artifact content and secret/private-path scan;
6. build metadata and source commit binding;
7. reproducibility comparison where the toolchain/platform permits it;
8. test-key verification during development.

A later separately reviewed GitHub release pipeline may add GitHub artifact
attestations or Sigstore keyless signatures using short-lived OIDC identity and
a public transparency log. That can provide provenance without a long-lived
private signing key, but it does not replace Apple notarization or Windows
Authenticode reputation.

## Key and password handling

Private keys, recovery codes, passwords, tokens, and certificate passphrases are
never printed into chat, committed, placed in repository fixtures, written to
vault notes, or passed on a command line. There is currently no production
release key.

If a long-lived independent release key is later justified:

- generate it directly in an operator-controlled encrypted environment;
- store the private key and passphrase in Bitwarden or hardware-backed custody;
- keep only the public key/fingerprint in the repository;
- create a separately stored revocation record;
- use a release-scoped subkey rather than the root where supported;
- rotate through a versioned trust policy without overwriting history.

The agent may provide public fingerprints and verification commands after key
generation, but it must not echo the secret key or passphrase back through chat.
Bitwarden entry creation remains an operator action unless a separately approved
credential broker can write the exact entry without exposing the secret.

## Artifact formats

Initial no-cost artifacts should be:

- Windows: portable `rz0.exe` ZIP first; an unsigned installer only after its
  install/uninstall/rollback behavior is fully exercised and warnings documented.
- macOS: architecture-specific tar archives and a universal archive when
  reproducibly available; optional unsigned/adhoc-signed DMG after clean-host
  tests.
- Linux: architecture-specific tar archives, then DEB and RPM packages; Arch
  `PKGBUILD`/package artifacts after pacman lifecycle tests.

Compatibility hosts install or unpack only these final artifacts. Build runners,
not user/test machines, carry compilers and packaging toolchains.

## Publication boundary

Release scripts and local artifact generation may be implemented and tested
without publishing. Creation of a GitHub workflow, draft/public GitHub Release,
package-channel submission, website deployment, or recurring automation remains
an explicit external-write/quota event and must record its exact capability
scope before execution.

See [`release-packaging.md`](release-packaging.md),
[`support-policy.md`](support-policy.md),
[`production-readiness.md`](production-readiness.md), and
[`SECURITY.md`](../SECURITY.md).
