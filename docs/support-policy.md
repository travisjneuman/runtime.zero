# Rolling Platform, Architecture, and Manager Support Policy

This policy freezes how the `runtime.zero 1.0` matrix expands without turning
“everything possible” into an untestable release claim. Windows, Windows Server,
macOS, and Linux remain equal release priorities. Support moves from newest to
older releases one generation at a time.

## Support tiers

- **Tier A — release-blocking:** the newest/current generation plus the three
  previous generations where artifacts, vendor media, and safe test environments
  are practically available. Every required module and lifecycle stage needs a
  final-artifact runtime result.
- **Tier B — compatibility:** older or vendor-retired generations tested when a
  safe image remains available. Failures are fixed when technically reasonable,
  but Tier B receives no security-support claim and cannot weaken Tier A.
- **Tier C — research:** additional distributions, architectures, managers,
  terminals, filesystems, and packaging systems admitted fixture-first, then
  promoted after repeatable artifact-only runtime proof.

A vendor-retired OS can never be described as secure or supported merely because
`rz0` starts. Its evidence is compatibility-only. Unsupported results must be
reported explicitly in CLI, JSON, TUI, and release documentation.

## Initial rolling matrix — 2026-07-29

The named versions are the current research baseline. At each release freeze,
official vendor lifecycle pages are rechecked and the four-generation window
rolls forward.

| Family | Tier-A/initial artifact matrix | Architectures |
| --- | --- | --- |
| Windows client | Windows 11 25H2, 24H2; 23H2 and 22H2 when safe images remain available, otherwise Tier B | x86-64 and ARM64 where the edition supports it |
| Windows Server LTSC | Server 2025, 2022, 2019, 2016 | x86-64; other vendor-supported architectures enter through Tier C |
| macOS | Tahoe 26, Sequoia 15, Sonoma 14, Ventura 13 | Apple Silicon and Intel where Apple supports the release/hardware pair |
| Ubuntu LTS | 26.04, 24.04, 22.04, 20.04 | x86-64 and ARM64 |
| Debian | 13 (Trixie), 12 (Bookworm), 11 (Bullseye), 10 (Buster) | x86-64 and ARM64 first; additional Debian architectures through Tier C |
| Red Hat Enterprise Linux | 10, 9, 8, 7 | x86-64 and ARM64 where vendor/release availability permits |
| Arch Linux | Current rolling repositories and installation image; prior three monthly snapshots are regression evidence, not separately supported releases | x86-64 first; Arch Linux ARM is a separate Tier-C downstream scope |

The oldest entries may already be outside normal vendor support. They remain in
the work-backwards compatibility queue but are automatically Tier B when the
vendor no longer provides ordinary security maintenance.

## Final-artifact-only runtime rule

A compatibility host should need only the artifact intended for users plus
standard OS facilities. It must not require a Rust toolchain, source checkout,
compiler, Visual Studio, Xcode, or repository scripts.

Build and test responsibilities are separate:

1. **Build runners** produce reproducible artifacts in controlled environments.
2. **Artifact verification** checks hashes, provenance, package contents, target
   ABI, and absence of private paths/secrets.
3. **Clean compatibility hosts** install or unpack only the final
   EXE/installer, macOS archive/DMG/package, or Linux archive/package.
4. **Runtime harnesses** exercise CLI, JSON, TUI, install/upgrade/rollback, and
   module behavior using public-safe synthetic fixtures.
5. **Disposable mutation hosts** are snapshot-backed and contain no personal,
   production, employer, or customer data.

Cross-compilation is build evidence only. A build runner is not runtime proof,
and a runtime host must not silently become a development workstation.

## Manager and platform-adapter order

Manager support is implemented in the foundation as normalized discovery,
plan, execution, transaction, and recovery adapters. Modules provide only their
domain rules.

1. Windows: WinGet, Windows Installer/MSI, MSIX/AppX, installed-application
   registry; then Chocolatey and Scoop.
2. macOS: Homebrew, application bundles, installer/pkg receipts, launchd; then
   MacPorts.
3. Ubuntu/Debian: APT/dpkg, systemd, Snap, Flatpak, and AppImage inventory.
4. RHEL: DNF/RPM and systemd; subscription-gated sources must work offline or
   report unavailable without requesting credentials.
5. Arch: pacman and systemd.
6. Tier C research: additional vendor managers, containers, language managers,
   filesystems, terminals, shells, and service systems that satisfy the same
   bounded fixture/runtime requirements.

No adapter may parse localized human output when a stable machine interface,
database, API, or manager-native query exists. Every adapter must pin an exact
executable, clear/allowlist environment, bound locale/input/output/time, detect
manager versions, define offline behavior, and preserve source-agreement and
rollback semantics.

## Terminals, filesystems, and privilege

The runtime matrix includes native terminals and shells first: Windows Terminal,
PowerShell 7, classic console; Terminal.app and common macOS terminal emulators;
and common Linux terminals with Bash and Zsh. Additional terminals are promoted
from Tier C after automated pseudo-terminal and manual accessibility evidence.

Each platform matrix covers its normal local filesystems plus symlink/reparse,
ACL/ownership, locked-file, case-sensitivity, long-path, low-space, read-only,
and cross-filesystem failure fixtures. Privileged behavior requires a separate
least-privilege adapter and must remain useful in report/dry-run mode without
elevation.

## Research and promotion rule

Long-term additions are recorded from primary vendor documentation. A target is
promoted only when its exact version, architecture, artifact format, manager,
terminal, fixture set, host identity, and evidence location are known. Discovery
never silently expands a 1.0 release blocker after RC freeze.

Primary baseline sources:

- [Windows 11 release information](https://learn.microsoft.com/en-us/windows/release-health/windows11-release-information)
- [Windows Server release information](https://learn.microsoft.com/en-us/windows-server/get-started/windows-server-release-info)
- [Apple macOS compatibility and current versions](https://support.apple.com/en-us/109033)
- [Ubuntu release cycle](https://ubuntu.com/about/release-cycle) and [release images](https://releases.ubuntu.com/)
- [Debian releases](https://www.debian.org/releases/)
- [Arch Linux rolling-release model](https://wiki.archlinux.org/title/Arch_Linux)
- [Red Hat Enterprise Linux life cycle](https://access.redhat.com/support/policy/updates/errata)

See [`production-readiness.md`](production-readiness.md) and
[`dependency-and-validation-audit.md`](dependency-and-validation-audit.md).
