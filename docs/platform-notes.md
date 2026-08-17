# Platform Notes

These notes describe current source behavior, not a frozen 1.0 support matrix.
Only the native development host is live-smoked in routine repository work;
cross-target compilation is useful evidence but does not prove runtime behavior.

## Capability summary

| Surface | macOS | Linux | Windows |
| --- | --- | --- | --- |
| Build target | Apple Silicon and Intel source targets | x86_64/aarch64 GNU source targets | x86_64 MSVC source target |
| Diagnostics/read-only scan | Implemented; native evidence exercised | Implemented; cross-compile/unit fixtures | Implemented; cross-compile/unit fixtures |
| Installed software metadata | `.app`, Homebrew roots, MacPorts roots, installer receipts | XDG desktop entries, dpkg status, pacman local metadata | persisted PATH, uninstall registry views |
| Service/persistence metadata | launchd plist labels | systemd unit-file labels | service registry metadata |
| Monitor depth | native host/memory/disk/network/process sample | native host/memory/disk/network/process sample | host/memory/disk/network/process sample; some CPU/depth limitations remain |
| Exact executable observation | vnode/open-file metadata and digest | open file, inode/device/link/mode/digest | handle/file identity and digest |
| Identity-to-spawn binding | **Blocked** | held `/proc/self/fd/<fd>` direct launch | **Blocked for production** |
| Manager apply | Fails closed before transaction | Narrow pre-alpha lane exists; not production-supported | Fails closed |
| Store creation | User-local scaffold implemented | User-local scaffold implemented | Blocked pending runtime ACL proof |
| Uninstall execution | Not implemented | Not implemented | Not implemented |
| Module lifecycle execution | Not implemented | Not implemented | Not implemented |

## macOS

### Current discovery

- standard user/system application roots;
- bundle `Info.plist` display name/version/bundle ID;
- Homebrew Cellar and Caskroom directory metadata without invoking `brew`;
- MacPorts metadata roots under `/opt/local/var/macports/software`;
- Apple Installer receipts under `/var/db/receipts` (bounded plist reads);
- launchd metadata from standard LaunchAgents/LaunchDaemons roots;
- allowlisted executable discovery and process PATH.

The collectors do not invoke `mdfind`, `pkgutil`, `launchctl`, or a manager.
Launchd labels and metadata paths are inventory evidence only; loaded/running
state is not asserted.

The explicit `rz0 updates --dry-run --all-providers --allow-network-read` lane
resolves provider ownership and adds bounded live availability probes for
Homebrew formulae and casks (greedy cask mode), MacPorts, Mac App Store `mas`
when installed, Apple `softwareupdate`, npm global prefixes, pip, RubyGems,
`rustup`, `uv`, Grok, Hermes, and oh-my-pi. It also audits observed-only Warp,
aiup, and Cargo channels, declared Electron GitHub metadata, and Sparkle app
bundles. A missing provider, parser drift, UI-only updater, or direct installer
is retained as an explicit warning. Unknown sources are never upgraded by a
guessed command, so this is broad provider coverage rather than a mathematical
claim of universal macOS support.

### Current mutation blocks

The updater can observe and seal a manager executable but cannot launch the exact
opened Mach-O identity using a reviewed primitive. Pathname spawn would reopen
the replacement race, so the action fails before transaction preparation.

Store creation uses user-local POSIX permission checks. No launch daemon,
privileged helper, Authorization Services flow, notarized package, or sandbox
profile exists. Disposable APFS snapshot/power-loss evidence remains required.

## Linux

### Current discovery

- process PATH plus persisted `/etc/environment` and shell profile PATH values;
- XDG desktop entries and explicit executable fields;
- direct `/var/lib/dpkg/status` package paragraphs;
- direct `/var/lib/pacman/local/*/desc` package metadata;
- systemd unit-file labels under standard system/user configuration roots;
- allowlisted manager/tool executables.

Direct package metadata reads avoid invoking apt/dpkg/pacman for baseline
inventory. Package status filters, parser ceilings, malformed-record handling,
and fixtures exist, but native distro/version proof is still required.
Systemd unit presence is not the same as enabled or active state; `enabled` is
therefore unknown in the current read-only record.

### Narrow update lane

On Linux, a regular single-link native ELF executable is opened and retained.
Scripts/interpreter chains fail before transaction preparation because a
close-on-exec descriptor path cannot yet bind both script and interpreter
identity. The process host launches `/proc/self/fd/<fd>` without shell/PATH resolution and validates
device/inode/mode/link/size/digest before and after spawn. The child receives a
dedicated process group, bounded output, deadlines, and cancellation teardown.

This does not provide a seccomp/namespace/cgroup/network policy. A manager may
spawn descendants or perform broad manager-native actions. No normal-workstation
apply is authorized; use disposable synthetic hosts only after explicit approval.

### Store

The user-local store initializer exists with create-new and mode checks. Native
NFS, unusual mount, quota, immutable-bit, full-disk, and power-loss matrices are
not complete.

## Windows

### Current discovery

- process PATH plus machine/user persisted PATH values;
- standard uninstall registry views, including 32/64-bit locations;
- product-code identifiers or stable SHA-256 registry-product-key digests;
- service/driver registry metadata under `CurrentControlSet\Services`;
- allowlisted executable observation using Win32 handle metadata.

Registry strings are bounded/sanitized. Windows service records are presence and
start-configuration evidence, not authoritative current-running state.

### Mutation blocks

Production apply remains disabled. The project still needs:

- exact inherited/duplicated executable handle-to-process-image binding;
- race-free Job Object assignment (suspended creation or equivalent);
- descendant escape/nested-job tests;
- reparse-safe owner/DACL directory traversal and creation;
- directory durability semantics and locked-file/pending-reboot handling;
- real Windows runner, cancellation, UAC, long-path, UNC, antivirus, and restart
  evidence.

Store creation, updater apply, and all cleanup/uninstall execution must fail
closed until those contracts have implementation and disposable-host proof.

## Cross-platform limitations

- No elevation broker exists.
- No uninstall executor exists.
- No direct cleanup/quarantine/restore executor exists.
- No production module process lifecycle exists.
- No first-party module is active/installed by the foundation.
- Native package signing/notarization/repository publication is incomplete.
- Runtime validation has not covered every locale, filesystem, architecture,
  manager version, or enterprise policy environment.
- Service/persistence inventory is intentionally metadata-only and does not yet
  reconcile live status, ownership, dependencies, or safe actionability.

## Evidence expectations

A target can advance only with final-artifact, target-native evidence. Required
release cells are tracked in [`release-acceptance.md`](release-acceptance.md) and
[`completion-checklist.md`](completion-checklist.md). Cross-compilation proves
that code type-checks for a target; it does not satisfy native behavior,
security, accessibility, recovery, packaging, or performance cells.
