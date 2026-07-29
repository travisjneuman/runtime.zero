# Windows and Windows Server Compatibility Matrix

This is the explicit newest-to-oldest Windows acceptance scope. It separates
product generations, editions, architectures, shell/terminal combinations, and
Rust artifact lanes so unsupported combinations are never guessed.

## Client generations and editions

| Generation | Microsoft-documented edition families in scope | Artifact lane |
| --- | --- | --- |
| Windows 11 | Home, Pro, Pro Education, Pro for Workstations, SE; Education, Enterprise, Enterprise multi-session; IoT Enterprise; N variants where Microsoft issued them | Ordinary x86-64/ARM64 MSVC artifacts; each real edition/architecture pair |
| Windows 10 | Home, Pro, Pro Education, Pro for Workstations; Education, Enterprise, Enterprise multi-session; IoT Enterprise and LTSC generations; N variants where issued | Ordinary x86/x86-64/ARM64 MSVC artifacts where supported; 22H2 plus obtainable LTSC/current images first, then historical releases |
| Windows 8.1 | Enterprise, Enterprise N, N, Pro with Media Center, Professional, Professional N, Single Language | Windows-7-baseline x86/x86-64 compatibility artifact and runtime proof required |
| Windows 8 | Enterprise, Enterprise N, N, Pro with Media Center, Professional, Professional N, Single Language | Windows-7-baseline x86/x86-64 compatibility artifact and runtime proof required |
| Windows 7 SP1 | Enterprise/Enterprise N, Home Basic, Home Premium/Home Premium N, Professional/Professional N, Professional for Embedded Systems, Starter/Starter N, Ultimate/Ultimate N, Ultimate for Embedded Systems | Windows-7-baseline x86/x86-64 compatibility artifact and isolated runtime proof required |

Edition names follow Microsoft lifecycle pages. Architecture, media, feature,
and edition combinations that Microsoft never shipped are recorded
`not_applicable`; they are not synthesized. Windows 10 feature releases and IoT/
LTSC variants receive a separate media census before an RC matrix is frozen.

## Server generations and editions

| Generation | Documented editions/variants to census | Initial artifact lane |
| --- | --- | --- |
| Server 2025 | Datacenter, Datacenter Azure Edition, Essentials, Standard; supported Core/Desktop forms | Ordinary x86-64 MSVC |
| Server 2022 | Datacenter, Datacenter Azure Edition, Essentials, Standard; supported Core/Desktop forms | Ordinary x86-64 MSVC |
| Server 2019 | Datacenter, Essentials, Standard; supported Core/Desktop forms | Ordinary x86-64 MSVC |
| Server 2016 | Datacenter, Essentials, MultiPoint Premium, Standard; Core/Desktop forms | Ordinary x86-64 MSVC baseline |
| Server 2012 R2 | Datacenter, Essentials, Embedded, Foundation, Standard; Core/GUI forms where offered | Windows-7-baseline x86-64 candidate; runtime proof required |
| Server 2012 | Datacenter, Essentials, External Connector, Embedded, Foundation, Standard; Core/GUI forms where offered | Windows-7-baseline x86-64 candidate; runtime proof required |
| Server 2008 R2 SP1 | Datacenter, Enterprise, HPC, Itanium, Standard, Web; Core/GUI forms where offered | Windows-7-baseline x86-64 candidate; Itanium requires a separate feasibility result |
| Server 2008 SP2 | Datacenter, Enterprise, Foundation, Standard, Web, without-Hyper-V variants, and Itanium | Ordinary modern Rust is not a valid baseline; x86/x86-64/Itanium each require a feasibility decision and isolated proof |

Historical Storage Server, Hyper-V Server, Small Business Server, and
Annual/Semi-Annual Channel products are tracked as additional variants when
lawful media and a meaningful user artifact exist. They do not disappear from
research merely because they are not editions on the primary lifecycle page.

## Rust and ABI constraint

The normal Rust `*-pc-windows-*` targets built with Rust 1.96 require Windows 10
or Server 2016. They cannot establish legacy compatibility.

Rust defines Tier-3 `x86_64-win7-windows-msvc` and
`i686-win7-windows-msvc` targets. They are not distributed by rustup, so a
controlled nightly/build-std Windows build runner with an appropriate SDK/linker
must produce candidate artifacts. These artifacts must retain the same source,
contracts, tests, package manifest, and security review as modern artifacts.
They may not pin the entire project to an obsolete Rust release.

The Windows-7 baseline is a candidate for Windows 8/8.1 and Server 2008 R2+
only after runtime proof. Server 2008 predates that baseline. A tiny legacy
launcher is acceptable only if it performs version/error reporting and launches
a compatible full artifact without implementing module, trust, transaction,
network, update, or cleanup behavior. Otherwise the matrix records a transparent
technically-impossible result.

## PowerShell coverage

The CLI and JSON surfaces are shell-independent, but quoting, pipeline,
redirection, exit-code, Unicode, completion, and install/uninstall instructions
must be exercised through:

- Windows PowerShell 1.0, 2.0, 3.0, 4.0, 5.0, and 5.1 on OS versions that
  actually shipped or support them;
- PowerShell Core 6.0, 6.1, and 6.2;
- PowerShell 7.0 through 7.6, including every obtainable LTS/stable minor line;
- current preview releases as non-release-blocking research;
- `cmd.exe` on every Windows generation.

Within each line, the latest patch is release-blocking and obtainable earlier
patches are boundary/regression evidence. Microsoft currently identifies 7.6 as
LTS, 7.5 as Stable, 7.4 as the prior supported LTS, and 7.7 as preview. Retired
PowerShell versions and retired Windows hosts are compatibility-only and always
isolated/offline unless a bounded download is required to create the image.

## Console and terminal coverage

- Classic Console Host is required on every generation.
- Windows Terminal packaged, preinstallation, unpackaged, and portable forms are
  tested on supported OS versions.
- Microsoft documents portable Windows Terminal as Windows 10 version 2004 or
  newer; it is not an acceptance requirement on Windows 7/8/8.1.
- Every obtainable stable Windows Terminal minor line enters the compatibility
  census; current stable is release-blocking, historical lines are regression
  evidence.
- Server Core uses non-GUI console/remoting surfaces appropriate to the image;
  lack of Windows Terminal is not a product failure.

## Test environment

Every retired Windows/Server image must be isolated, snapshot-backed, free of
personal/production data, and denied general network access after required media
and artifacts are staged. The host receives only the final ZIP/EXE/installer and
public synthetic fixtures. Build tools remain on dedicated build runners.

## Primary sources

- [Rust Windows baseline change](https://blog.rust-lang.org/2024/02/26/Windows-7/)
- [Rust platform support](https://doc.rust-lang.org/rustc/platform-support.html)
- [Windows 11 Home/Pro lifecycle](https://learn.microsoft.com/en-us/lifecycle/products/windows-11-home-and-pro)
- [Windows 11 Enterprise/Education lifecycle](https://learn.microsoft.com/en-us/lifecycle/products/windows-11-enterprise-and-education)
- [Windows 10 Home/Pro lifecycle](https://learn.microsoft.com/en-us/lifecycle/products/windows-10-home-and-pro)
- [Windows 10 Enterprise/Education lifecycle](https://learn.microsoft.com/en-us/lifecycle/products/windows-10-enterprise-and-education)
- [Windows 8.1 lifecycle](https://learn.microsoft.com/en-us/lifecycle/products/windows-81)
- [Windows 8 lifecycle](https://learn.microsoft.com/en-us/lifecycle/products/windows-8)
- [Windows 7 lifecycle](https://learn.microsoft.com/en-us/lifecycle/products/windows-7)
- [Windows Server release information](https://learn.microsoft.com/en-us/windows-server/get-started/windows-server-release-info)
- [PowerShell support lifecycle](https://learn.microsoft.com/en-us/powershell/scripting/install/powershell-support-lifecycle)
- [Windows Terminal distributions](https://learn.microsoft.com/en-us/windows/terminal/distributions)

See [`support-policy.md`](support-policy.md),
[`release-packaging.md`](release-packaging.md), and
[`production-readiness.md`](production-readiness.md).
