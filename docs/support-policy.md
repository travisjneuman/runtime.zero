# Rolling Platform, Architecture, and Manager Support Policy

This policy freezes how the `runtime.zero 1.0` matrix expands without turning
“everything possible” into an untestable release claim. Windows, Windows Server,
macOS, and Linux remain equal release priorities. Support moves from newest to
older releases one generation at a time.

## Support tiers

- **Tier A — release-blocking supported systems:** every in-scope generation
  still receiving ordinary vendor security maintenance. Every required module
  and lifecycle stage needs a final-artifact runtime result.
- **Tier B — mandatory legacy compatibility:** Windows 8.1, 8, 7 and Server 2008
  through 2012 R2, plus any other named generation that has left normal vendor
  support. Each requires an explicit artifact result when lawful media and an
  isolated host are practically available. Tier B receives no security-support
  claim and cannot weaken Tier A.
- **Tier C — research:** additional distributions, architectures, managers,
  terminals, filesystems, and packaging systems admitted fixture-first, then
  promoted after repeatable artifact-only runtime proof.

A vendor-retired OS can never be described as secure or supported merely because
`rz0` starts. Its evidence is compatibility-only. Unsupported results must be
reported explicitly in CLI, JSON, TUI, and release documentation.

## Initial rolling matrix — 2026-07-29

The named versions are the current research baseline. At each release freeze,
official vendor lifecycle pages are rechecked. Windows client generations are
explicitly 11, 10, 8.1, 8, and 7—not feature-update arithmetic.

| Family | Initial artifact matrix | Architectures |
| --- | --- | --- |
| Windows client | Windows 11 and 10 across available Home, Pro, Enterprise, Education, N, workstation, IoT/embedded, and architecture-relevant variants; Windows 8.1, 8, and 7 across every Microsoft-documented edition as mandatory Tier-B compatibility | x86-64 and ARM64 where supported; x86 for legacy editions; edition/architecture combinations that never existed are recorded not-applicable |
| Windows Server | Server 2025, 2022, 2019, 2016, 2012 R2, 2012, 2008 R2, and 2008; Standard/Datacenter/Essentials/Foundation/Web/HPC/Storage/Itanium and Core/Desktop variants where that release actually offered them; historical Annual/Semi-Annual channels remain compatibility research | x86-64 first; x86 and Itanium variants are evidence/research targets only where a compatible artifact can technically and lawfully be produced |
| macOS | Tahoe 26, Sequoia 15, Sonoma 14, Ventura 13, then older releases one by one through Tier C | Apple Silicon and Intel where Apple supports the release/hardware pair |
| Ubuntu LTS | 26.04, 24.04, 22.04, 20.04, then older LTS releases one by one | x86-64 and ARM64 |
| Debian | 13 (Trixie), 12 (Bookworm), 11 (Bullseye), 10 (Buster), then older releases one by one | x86-64 and ARM64 first; additional Debian architectures through Tier C |
| Red Hat Enterprise Linux | 10, 9, 8, 7, then older available major releases one by one | x86-64 and ARM64 where vendor/release availability permits |
| Arch Linux | Current rolling repositories and installation image; prior monthly snapshots are regression evidence, not separately supported releases | x86-64 first; Arch Linux ARM is a separate Tier-C downstream scope |

Every edition/variant is cataloged against its real product generation rather
than guessed. “All variants” never means testing impossible combinations. The
oldest entries remain mandatory compatibility investigations but are Tier B when
the vendor no longer provides ordinary security maintenance.

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

## Legacy Windows toolchain boundary

Rust 1.78 raised the ordinary `*-pc-windows-*` binary baseline to Windows 10;
the current Rust platform table also describes standard MSVC x86/x86-64 as
Windows 10+/Server 2016+. Therefore the normal `runtime.zero` artifact cannot be
assumed to run on Windows 8.1/8/7 or Server 2012/2008.

Rust defines Tier-3 `x86_64-win7-windows-msvc` and
`i686-win7-windows-msvc` targets, but rustup does not currently distribute their
standard libraries. A separate controlled build-std/link lane and clean-host
runtime proof are required. A Windows-7-baseline artifact may cover 8/8.1 after
proof; it does not establish Server 2008 compatibility. Server 2008 and Itanium
may require a small legacy launcher, a separately maintained compatibility
artifact, or an explicit technically-impossible result. None may lower the
modern core's Rust/security standards or be silently represented as supported.

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

## Shell and terminal matrix

Windows shell coverage works backward through PowerShell 7.6/7.5/7.4 and every
obtainable retired 7.x line, PowerShell Core 6.x, Windows PowerShell 5.1 through
1.0 where the matching OS shipped/supports it, and `cmd.exe`. Preview PowerShell
builds are research-only. A PowerShell version/OS combination that Microsoft or
its .NET runtime never supported is not-applicable, not a failed test.

Windows terminal coverage includes classic console hosts and every obtainable
stable Windows Terminal line on an OS it supports. Microsoft documents portable
Windows Terminal as Windows 10 version 2004 or newer, so it is not fabricated as
a Windows 7/8 test requirement.

macOS and Linux terminal coverage is an open compatibility census rather than a
finite claim that every terminal ever released exists in the lab. It starts with
all OS-bundled Terminal.app generations in the macOS matrix and current plus
obtainable historical versions of iTerm2, Ghostty, Kitty, WezTerm, Alacritty,
GNOME Terminal, Konsole, xterm, foot, Terminator, Tilix, and Xfce Terminal.
Additional emulators discovered through research are added continuously. Each
is tested through its real PTY, keyboard, resize, Unicode, color, alternate-
screen, restore, pipe/redirection, and accessibility behavior; noninteractive
CLI/JSON correctness remains terminal-independent.

## Filesystems and privilege

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
- [Windows 8.1 lifecycle](https://learn.microsoft.com/en-us/lifecycle/products/windows-81), [Windows 8 lifecycle](https://learn.microsoft.com/en-us/lifecycle/products/windows-8), and [Windows 7 lifecycle](https://learn.microsoft.com/en-us/lifecycle/products/windows-7)
- [Rust Windows baseline change and Windows 7 targets](https://blog.rust-lang.org/2024/02/26/Windows-7/)
- [PowerShell support lifecycle](https://learn.microsoft.com/en-us/powershell/scripting/install/powershell-support-lifecycle)
- [Windows Terminal distributions](https://learn.microsoft.com/en-us/windows/terminal/distributions)
- [Apple macOS compatibility and current versions](https://support.apple.com/en-us/109033)
- [Ubuntu release cycle](https://ubuntu.com/about/release-cycle) and [release images](https://releases.ubuntu.com/)
- [Debian releases](https://www.debian.org/releases/)
- [Arch Linux rolling-release model](https://wiki.archlinux.org/title/Arch_Linux)
- [Red Hat Enterprise Linux life cycle](https://access.redhat.com/support/policy/updates/errata)

See [`windows-compatibility.md`](windows-compatibility.md),
[`production-readiness.md`](production-readiness.md), and
[`dependency-and-validation-audit.md`](dependency-and-validation-audit.md).
