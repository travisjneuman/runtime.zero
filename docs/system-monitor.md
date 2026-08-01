# Built-in System Monitor

`rz0 monitor` and the TUI's `m` section provide a native, read-only system
monitor. This is the runtime.zero equivalent of btop: it is part of the
installed foundation and does not require btop, top, task-manager, PowerShell,
or another separately installed program.

The monitor reports:

- CPU sampling, load averages, and logical CPU count;
- physical memory and swap where the platform exposes them;
- mounted disks and used/available space;
- network interface count and byte counters where available;
- process count, running count, and bounded top-process rows by CPU or memory;
- uptime and bounded platform warnings.

`rz0 monitor` is a one-shot scriptable snapshot:

```text
rz0 monitor
rz0 monitor --format json
```

Bare `rz0` refreshes the monitor section once per second while the TUI is open.
`m` jumps directly to that section. The monitor never writes, invokes a shell,
contacts a network source, changes process state, or executes a manager.

## Platform backends

The user-facing contract is shared; the collector uses the operating system's
stable local interfaces rather than parsing a third-party monitor's output.

| Platform family | Native sources | Coverage posture |
| --- | --- | --- |
| macOS | Mach host statistics, `sysctl`, `libproc`, `getifaddrs`, and `statvfs` | CPU, memory, disk, process, load, uptime, and interface counts are native; optional counters report a visible warning when unavailable |
| Linux distributions | `/proc/stat`, `/proc/meminfo`, `/proc/loadavg`, `/proc/net/dev`, `/proc/<pid>`, `statvfs`, and kernel CPU affinity data | The same collector works across distributions because it targets the Linux kernel ABI, not systemd, GNOME, KDE, a package manager, or a vendor utility |
| Windows | Kernel32 system/memory/disk APIs, Toolhelp process enumeration, PSAPI working-set data, and IP Helper interface APIs | Uses APIs available to the supported Windows baseline; it does not require PowerShell, WMI, Terminal, Task Manager, or a separately installed utility |

“All Linux variants” means supported Linux kernels and normal container/host
layouts, not a promise that a restricted container exposes host `/proc` or
network counters. Missing mounts and denied process records remain a valid
snapshot with explicit warnings.

“All Windows versions” must be tied to a published support baseline. The
foundation targets the repository's Windows 7/Server 2008 R2-and-newer matrix;
Windows editions below that baseline cannot be claimed without a separate Rust
runtime and API validation pass. The collector deliberately uses older Win32
entry points where possible so Windows 7, 8/8.1, 10, 11, and corresponding
Server editions share one path.

## Product boundary

A monitor is observation, not remediation. It may later become the evidence
source for a separately reviewed action such as terminating a process, changing
priority, stopping a service, or managing disk pressure, but those are different
operations with different confirmation, privilege, rollback, and recovery
contracts. The monitor itself must not grow a hidden kill/renice/cleanup key.

The snapshot contract is `system_monitor_snapshot`, schema 1, with
`read_only: true` and `writes_attempted: false`. A platform-specific metric must
be omitted or marked unavailable with a warning rather than guessed from a
locale-sensitive command or silently reported as zero.
