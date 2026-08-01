#[cfg(target_os = "macos")]
use std::collections::BTreeSet;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::ffi::CString;
use std::fmt::Write as _;
#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "macos")]
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

pub const SYSTEM_MONITOR_CONTRACT: &str = "system_monitor_snapshot";
const SCHEMA_VERSION: u16 = 1;
const MAX_PROCESS_ROWS: usize = 8;
#[cfg(target_os = "linux")]
const MAX_PROCESS_SAMPLES: usize = 4096;
const MAX_WARNINGS: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SystemSnapshot {
    pub schema_version: u16,
    pub contract: &'static str,
    pub read_only: bool,
    pub writes_attempted: bool,
    pub platform: &'static str,
    pub collected_at_unix_seconds: u64,
    pub uptime_seconds: Option<u64>,
    pub cpu: CpuSnapshot,
    pub memory: MemorySnapshot,
    pub disks: Vec<DiskSnapshot>,
    pub network: NetworkSnapshot,
    pub processes: ProcessSummary,
    pub warnings: Vec<String>,
    #[serde(skip)]
    cpu_total: Option<u64>,
    #[serde(skip)]
    cpu_idle: Option<u64>,
    #[serde(skip)]
    process_samples: Vec<ProcessSample>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CpuSnapshot {
    pub usage_percent: Option<u16>,
    pub load_average_milli: [Option<u32>; 3],
    pub logical_cpus: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MemorySnapshot {
    pub total_bytes: Option<u64>,
    pub used_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
    pub swap_total_bytes: Option<u64>,
    pub swap_used_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiskSnapshot {
    pub mount: String,
    pub total_bytes: Option<u64>,
    pub used_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NetworkSnapshot {
    pub interface_count: usize,
    pub received_bytes: Option<u64>,
    pub transmitted_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProcessSummary {
    pub total: usize,
    pub running: usize,
    pub top: Vec<ProcessRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProcessRow {
    pub pid: u32,
    pub name: String,
    pub state: Option<String>,
    pub cpu_percent: Option<u16>,
    pub memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProcessSample {
    pid: u32,
    cpu_ticks: u64,
}

pub fn collect_snapshot(previous: Option<&SystemSnapshot>) -> SystemSnapshot {
    #[cfg(target_os = "linux")]
    let mut snapshot = collect_linux(previous);
    #[cfg(target_os = "macos")]
    let mut snapshot = collect_macos(previous);
    #[cfg(windows)]
    let mut snapshot = collect_windows(previous);
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    let mut snapshot = collect_unsupported();

    snapshot.warnings.truncate(MAX_WARNINGS);
    snapshot
}

pub fn render_text(snapshot: &SystemSnapshot) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "runtime.zero system monitor");
    let _ = writeln!(output, "contract: {}", snapshot.contract);
    let _ = writeln!(output, "platform: {}", snapshot.platform);
    let _ = writeln!(
        output,
        "uptime: {}",
        snapshot
            .uptime_seconds
            .map(format_duration)
            .unwrap_or_else(|| "unavailable".to_string())
    );
    let _ = writeln!(
        output,
        "cpu: {} · load: {} · logical cpus: {}",
        snapshot
            .cpu
            .usage_percent
            .map(|value| format!("{value}%"))
            .unwrap_or_else(|| "sampling".to_string()),
        format_load(snapshot.cpu.load_average_milli),
        snapshot
            .cpu
            .logical_cpus
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unavailable".to_string())
    );
    let _ = writeln!(
        output,
        "memory: {} used / {} total · {} available",
        format_bytes(snapshot.memory.used_bytes),
        format_bytes(snapshot.memory.total_bytes),
        format_bytes(snapshot.memory.available_bytes)
    );
    let _ = writeln!(
        output,
        "swap: {} used / {} total",
        format_bytes(snapshot.memory.swap_used_bytes),
        format_bytes(snapshot.memory.swap_total_bytes)
    );
    for disk in &snapshot.disks {
        let _ = writeln!(
            output,
            "disk {}: {} used / {} total · {} available",
            disk.mount,
            format_bytes(disk.used_bytes),
            format_bytes(disk.total_bytes),
            format_bytes(disk.available_bytes)
        );
    }
    let _ = writeln!(
        output,
        "network: {} interfaces · received {} · transmitted {}",
        snapshot.network.interface_count,
        format_bytes(snapshot.network.received_bytes),
        format_bytes(snapshot.network.transmitted_bytes)
    );
    let _ = writeln!(
        output,
        "processes: {} total · {} running",
        snapshot.processes.total, snapshot.processes.running
    );
    if !snapshot.processes.top.is_empty() {
        output.push_str("top processes:\n");
        for process in &snapshot.processes.top {
            let _ = writeln!(
                output,
                "  {:>6} {:<24} cpu={} memory={} state={}",
                process.pid,
                process.name,
                process
                    .cpu_percent
                    .map(|value| format!("{value}%"))
                    .unwrap_or_else(|| "sampling".to_string()),
                format_bytes(process.memory_bytes),
                process.state.as_deref().unwrap_or("?")
            );
        }
    }
    if !snapshot.warnings.is_empty() {
        output.push_str("warnings:\n");
        for warning in &snapshot.warnings {
            let _ = writeln!(output, "  - {warning}");
        }
    }
    output.push_str("writes_attempted: false\n");
    output
}

pub fn render_json(snapshot: &SystemSnapshot) -> Result<String, String> {
    serde_json::to_string_pretty(snapshot)
        .map(|json| format!("{json}\n"))
        .map_err(|error| format!("serialize system monitor snapshot: {error}"))
}

pub fn monitor_command(args: &[String]) -> (crate::ExitCode, String, String) {
    let mut json = false;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--help" | "-h" | "help" if args.len() == 1 => {
                return (crate::ExitCode::Ok, monitor_usage(), String::new());
            }
            "--json" => json = true,
            "--format" => {
                let Some(value) = args.get(index + 1).map(String::as_str) else {
                    return monitor_usage_error("--format requires text or json");
                };
                match value {
                    "text" => json = false,
                    "json" => json = true,
                    _ => return monitor_usage_error("--format requires text or json"),
                }
                index += 1;
            }
            value => return monitor_usage_error(&format!("unsupported monitor option '{value}'")),
        }
        index += 1;
    }
    let snapshot = collect_snapshot(None);
    let output = if json {
        match render_json(&snapshot) {
            Ok(output) => output,
            Err(error) => return (crate::ExitCode::Usage, String::new(), format!("{error}\n")),
        }
    } else {
        render_text(&snapshot)
    };
    (crate::ExitCode::Ok, output, String::new())
}

fn monitor_usage_error(message: &str) -> (crate::ExitCode, String, String) {
    (
        crate::ExitCode::Usage,
        String::new(),
        format!("{message}\n\n{}", monitor_usage()),
    )
}

fn monitor_usage() -> String {
    "Usage: rz0 monitor [--format text|json] [--json]\n\nReads native CPU, memory, disk, network, uptime, and process counters without changing the host.\nThe interactive `rz0` dashboard refreshes the monitor section automatically.".to_string()
}

pub fn format_bytes(value: Option<u64>) -> String {
    let Some(value) = value else {
        return "unavailable".to_string();
    };
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut amount = value as f64;
    let mut unit = 0usize;
    while amount >= 1024.0 && unit < UNITS.len() - 1 {
        amount /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value} B")
    } else {
        format!("{amount:.1} {}", UNITS[unit])
    }
}

pub fn format_duration(seconds: u64) -> String {
    let days = seconds / 86_400;
    let hours = seconds / 3_600 % 24;
    let minutes = seconds / 60 % 60;
    let seconds = seconds % 60;
    if days > 0 {
        format!("{days}d {hours:02}h {minutes:02}m")
    } else {
        format!("{hours:02}h {minutes:02}m {seconds:02}s")
    }
}

fn base_snapshot(platform: &'static str) -> SystemSnapshot {
    SystemSnapshot {
        schema_version: SCHEMA_VERSION,
        contract: SYSTEM_MONITOR_CONTRACT,
        read_only: true,
        writes_attempted: false,
        platform,
        collected_at_unix_seconds: unix_seconds(),
        uptime_seconds: None,
        cpu: CpuSnapshot {
            usage_percent: None,
            load_average_milli: [None, None, None],
            logical_cpus: logical_cpus(),
        },
        memory: MemorySnapshot {
            total_bytes: None,
            used_bytes: None,
            available_bytes: None,
            swap_total_bytes: None,
            swap_used_bytes: None,
        },
        disks: Vec::new(),
        network: NetworkSnapshot {
            interface_count: 0,
            received_bytes: None,
            transmitted_bytes: None,
        },
        processes: ProcessSummary {
            total: 0,
            running: 0,
            top: Vec::new(),
        },
        warnings: Vec::new(),
        cpu_total: None,
        cpu_idle: None,
        process_samples: Vec::new(),
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn collect_unsupported() -> SystemSnapshot {
    let mut snapshot = base_snapshot(std::env::consts::OS);
    snapshot
        .warnings
        .push("system monitor has no native collector for this platform".to_string());
    snapshot
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn logical_cpus() -> Option<u16> {
    std::thread::available_parallelism()
        .ok()
        .and_then(|value| u16::try_from(value.get()).ok())
}

fn format_load(load: [Option<u32>; 3]) -> String {
    load.iter()
        .map(|value| {
            value
                .map(|value| format!("{}.{:03}", value / 1000, value % 1000))
                .unwrap_or_else(|| "?".to_string())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn load_from_values(values: [f64; 3]) -> [Option<u32>; 3] {
    values.map(|value| {
        if value.is_finite() && value >= 0.0 {
            Some((value * 1000.0).round().min(u32::MAX as f64) as u32)
        } else {
            None
        }
    })
}

fn cpu_usage(
    previous: Option<&SystemSnapshot>,
    total: Option<u64>,
    idle: Option<u64>,
) -> Option<u16> {
    let previous_total = previous.and_then(|snapshot| snapshot.cpu_total)?;
    let previous_idle = previous.and_then(|snapshot| snapshot.cpu_idle)?;
    let total_delta = total?.checked_sub(previous_total)?;
    let idle_delta = idle?.checked_sub(previous_idle)?.min(total_delta);
    if total_delta == 0 {
        return None;
    }
    Some(((total_delta - idle_delta).saturating_mul(100) / total_delta).min(u16::MAX as u64) as u16)
}

fn add_warning(snapshot: &mut SystemSnapshot, message: impl Into<String>) {
    if snapshot.warnings.len() < MAX_WARNINGS {
        snapshot.warnings.push(message.into());
    }
}

#[cfg(target_os = "linux")]
fn collect_linux(previous: Option<&SystemSnapshot>) -> SystemSnapshot {
    let mut snapshot = base_snapshot("linux");
    let cpu_text = fs::read_to_string("/proc/stat");
    let (cpu_total, cpu_idle) = match cpu_text {
        Ok(text) => parse_linux_cpu(&text),
        Err(error) => {
            add_warning(&mut snapshot, format!("read /proc/stat: {error}"));
            (None, None)
        }
    };
    snapshot.cpu_total = cpu_total;
    snapshot.cpu_idle = cpu_idle;
    snapshot.cpu.usage_percent = cpu_usage(previous, cpu_total, cpu_idle);
    if let Ok(text) = fs::read_to_string("/proc/loadavg") {
        let values = text
            .split_whitespace()
            .take(3)
            .map(|value| value.parse::<f64>().ok().unwrap_or(0.0))
            .collect::<Vec<_>>();
        if values.len() == 3 {
            snapshot.cpu.load_average_milli = load_from_values([values[0], values[1], values[2]]);
        }
    } else {
        add_warning(&mut snapshot, "load average is unavailable");
    }
    if let Ok(text) = fs::read_to_string("/proc/uptime") {
        snapshot.uptime_seconds = text
            .split_whitespace()
            .next()
            .and_then(|value| value.parse::<f64>().ok())
            .map(|value| value.max(0.0) as u64);
    }
    parse_linux_memory(&mut snapshot);
    snapshot.disks.extend(unix_disks(&["/"]));
    parse_linux_network(&mut snapshot);
    collect_linux_processes(&mut snapshot, previous);
    snapshot
}

#[cfg(target_os = "linux")]
fn parse_linux_cpu(text: &str) -> (Option<u64>, Option<u64>) {
    let Some(line) = text.lines().find(|line| line.starts_with("cpu ")) else {
        return (None, None);
    };
    let values = line
        .split_whitespace()
        .skip(1)
        .filter_map(|value| value.parse::<u64>().ok())
        .collect::<Vec<_>>();
    if values.len() < 4 {
        return (None, None);
    }
    let total = values.iter().copied().sum();
    let idle = values[3].saturating_add(values.get(4).copied().unwrap_or(0));
    (Some(total), Some(idle))
}

#[cfg(target_os = "linux")]
fn parse_linux_memory(snapshot: &mut SystemSnapshot) {
    let Ok(text) = fs::read_to_string("/proc/meminfo") else {
        add_warning(snapshot, "memory information is unavailable");
        return;
    };
    let mut values = std::collections::BTreeMap::new();
    for line in text.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let Some(value) = value
            .split_whitespace()
            .next()
            .and_then(|v| v.parse::<u64>().ok())
        else {
            continue;
        };
        values.insert(name, value.saturating_mul(1024));
    }
    snapshot.memory.total_bytes = values.get("MemTotal").copied();
    snapshot.memory.available_bytes = values
        .get("MemAvailable")
        .copied()
        .or_else(|| values.get("MemFree").copied());
    snapshot.memory.used_bytes = snapshot
        .memory
        .total_bytes
        .zip(snapshot.memory.available_bytes)
        .map(|(total, available)| total.saturating_sub(available));
    snapshot.memory.swap_total_bytes = values.get("SwapTotal").copied();
    snapshot.memory.swap_used_bytes = values
        .get("SwapTotal")
        .copied()
        .zip(values.get("SwapFree").copied())
        .map(|(total, free)| total.saturating_sub(free));
}

#[cfg(target_os = "linux")]
fn parse_linux_network(snapshot: &mut SystemSnapshot) {
    let Ok(text) = fs::read_to_string("/proc/net/dev") else {
        add_warning(snapshot, "network counters are unavailable");
        return;
    };
    let mut received = 0u64;
    let mut transmitted = 0u64;
    let mut interfaces = 0usize;
    for line in text.lines().skip(2) {
        let Some((_, values)) = line.split_once(':') else {
            continue;
        };
        let fields = values.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 9 {
            continue;
        }
        interfaces += 1;
        received = received.saturating_add(fields[0].parse::<u64>().ok().unwrap_or(0));
        transmitted = transmitted.saturating_add(fields[8].parse::<u64>().ok().unwrap_or(0));
    }
    snapshot.network = NetworkSnapshot {
        interface_count: interfaces,
        received_bytes: Some(received),
        transmitted_bytes: Some(transmitted),
    };
}

#[cfg(target_os = "linux")]
fn collect_linux_processes(snapshot: &mut SystemSnapshot, previous: Option<&SystemSnapshot>) {
    let previous_samples = previous
        .map(|snapshot| snapshot.process_samples.as_slice())
        .unwrap_or(&[]);
    let total_delta = snapshot
        .cpu_total
        .zip(previous.and_then(|value| value.cpu_total))
        .and_then(|(current, old)| current.checked_sub(old));
    let logical = u64::from(snapshot.cpu.logical_cpus.unwrap_or(1));
    let mut rows = Vec::new();
    let mut samples = Vec::new();
    let Ok(entries) = fs::read_dir("/proc") else {
        add_warning(snapshot, "process information is unavailable");
        return;
    };
    let mut running = 0usize;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name.to_str().and_then(|value| value.parse::<u32>().ok()) else {
            continue;
        };
        if samples.len() >= MAX_PROCESS_SAMPLES {
            break;
        }
        let stat = match fs::read_to_string(entry.path().join("stat")) {
            Ok(stat) => stat,
            Err(_) => continue,
        };
        let Some((_, tail)) = stat.rsplit_once(')') else {
            continue;
        };
        let fields = tail.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 22 {
            continue;
        }
        let state = fields[0].to_string();
        if state == "R" {
            running += 1;
        }
        let user_ticks = fields[11].parse::<u64>().ok().unwrap_or(0);
        let system_ticks = fields[12].parse::<u64>().ok().unwrap_or(0);
        let ticks = user_ticks.saturating_add(system_ticks);
        samples.push(ProcessSample {
            pid,
            cpu_ticks: ticks,
        });
        let previous_ticks = previous_samples
            .iter()
            .find(|sample| sample.pid == pid)
            .map(|sample| sample.cpu_ticks);
        let cpu_percent = total_delta
            .zip(previous_ticks)
            .and_then(|(total_delta, previous_ticks)| {
                ticks
                    .checked_sub(previous_ticks)
                    .map(|delta| (delta, total_delta))
            })
            .and_then(|(delta, total_delta)| {
                (total_delta > 0).then(|| {
                    delta
                        .saturating_mul(logical)
                        .saturating_mul(100)
                        .checked_div(total_delta)
                        .unwrap_or(0)
                        .min(u16::MAX as u64) as u16
                })
            });
        let memory_bytes = fields[21]
            .parse::<u64>()
            .ok()
            .map(|pages| pages.saturating_mul(page_size()));
        let process_name = fs::read_to_string(entry.path().join("comm"))
            .ok()
            .map(|value| sanitize_name(value.trim()))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| pid.to_string());
        rows.push(ProcessRow {
            pid,
            name: process_name,
            state: Some(state),
            cpu_percent,
            memory_bytes,
        });
    }
    rows.sort_by(|left, right| {
        right
            .cpu_percent
            .unwrap_or(0)
            .cmp(&left.cpu_percent.unwrap_or(0))
            .then_with(|| {
                right
                    .memory_bytes
                    .unwrap_or(0)
                    .cmp(&left.memory_bytes.unwrap_or(0))
            })
            .then_with(|| left.pid.cmp(&right.pid))
    });
    snapshot.processes = ProcessSummary {
        total: rows.len(),
        running,
        top: rows.into_iter().take(MAX_PROCESS_ROWS).collect(),
    };
    snapshot.process_samples = samples;
}

#[cfg(target_os = "linux")]
fn page_size() -> u64 {
    unsafe { libc::sysconf(libc::_SC_PAGESIZE) }
        .try_into()
        .ok()
        .filter(|value: &u64| *value > 0)
        .unwrap_or(4096)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn unix_disks(paths: &[&str]) -> Vec<DiskSnapshot> {
    paths.iter().filter_map(|path| unix_disk(path)).collect()
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn unix_disk(path: &str) -> Option<DiskSnapshot> {
    let path = CString::new(path).ok()?;
    let mut stats = unsafe { std::mem::zeroed::<libc::statvfs>() };
    if unsafe { libc::statvfs(path.as_ptr(), &mut stats) } != 0 {
        return None;
    }
    let block_size = stats.f_frsize.max(stats.f_bsize);
    let total = u64::from(stats.f_blocks).saturating_mul(block_size);
    let available = u64::from(stats.f_bavail).saturating_mul(block_size);
    Some(DiskSnapshot {
        mount: path.to_string_lossy().into_owned(),
        total_bytes: Some(total),
        used_bytes: Some(total.saturating_sub(available)),
        available_bytes: Some(available),
    })
}

#[cfg(target_os = "macos")]
fn collect_macos(previous: Option<&SystemSnapshot>) -> SystemSnapshot {
    let mut snapshot = base_snapshot("macos");
    let (cpu_total, cpu_idle) = mac_cpu_times();
    snapshot.cpu_total = cpu_total;
    snapshot.cpu_idle = cpu_idle;
    snapshot.cpu.usage_percent = cpu_usage(previous, cpu_total, cpu_idle);
    snapshot.cpu.load_average_milli = mac_load_average();
    snapshot.uptime_seconds = mac_uptime_seconds();
    parse_macos_memory(&mut snapshot);
    snapshot.disks.extend(unix_disks(&["/"]));
    mac_network(&mut snapshot);
    mac_processes(&mut snapshot);
    snapshot
}

#[cfg(target_os = "macos")]
fn sysctl_bytes(name: &str) -> Option<Vec<u8>> {
    let name = CString::new(name).ok()?;
    let mut length = 0usize;
    let result = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            std::ptr::null_mut(),
            &mut length,
            std::ptr::null_mut(),
            0,
        )
    };
    if result != 0 || length == 0 || length > 1024 * 1024 {
        return None;
    }
    let mut bytes = vec![0u8; length];
    let result = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            bytes.as_mut_ptr().cast(),
            &mut length,
            std::ptr::null_mut(),
            0,
        )
    };
    if result != 0 {
        None
    } else {
        bytes.truncate(length);
        Some(bytes)
    }
}

#[cfg(target_os = "macos")]
fn sysctl_u64(name: &str) -> Option<u64> {
    let bytes = sysctl_bytes(name)?;
    (bytes.len() >= std::mem::size_of::<u64>())
        .then(|| u64::from_ne_bytes(bytes[..8].try_into().unwrap_or([0; 8])))
}

#[cfg(target_os = "macos")]
fn mac_cpu_times() -> (Option<u64>, Option<u64>) {
    let Some(bytes) = sysctl_bytes("kern.cp_time") else {
        return (None, None);
    };
    let width = std::mem::size_of::<libc::c_ulong>();
    if bytes.len() < width * 4 {
        return (None, None);
    }
    let values = (0..bytes.len() / width)
        .map(|index| {
            if width == 8 {
                u64::from_ne_bytes(
                    bytes[index * width..index * width + width]
                        .try_into()
                        .unwrap_or([0; 8]),
                )
            } else {
                u32::from_ne_bytes(
                    bytes[index * width..index * width + width]
                        .try_into()
                        .unwrap_or([0; 4]),
                ) as u64
            }
        })
        .collect::<Vec<_>>();
    let total = values.iter().copied().sum();
    let idle = values.get(3).copied().unwrap_or(0);
    (Some(total), Some(idle))
}

#[cfg(target_os = "macos")]
fn mac_load_average() -> [Option<u32>; 3] {
    let mut values = [0.0f64; 3];
    let count = unsafe { libc::getloadavg(values.as_mut_ptr(), 3) };
    if count == 3 {
        load_from_values(values)
    } else {
        [None, None, None]
    }
}

#[cfg(target_os = "macos")]
fn mac_uptime_seconds() -> Option<u64> {
    let bytes = sysctl_bytes("kern.boottime")?;
    if bytes.len() < 8 {
        return None;
    }
    let boot = i64::from_ne_bytes(bytes[..8].try_into().ok()?).max(0) as u64;
    Some(unix_seconds().saturating_sub(boot))
}

#[cfg(target_os = "macos")]
fn parse_macos_memory(snapshot: &mut SystemSnapshot) {
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct VmStatistics64 {
        free_count: u32,
        active_count: u32,
        inactive_count: u32,
        wire_count: u32,
        zero_fill_count: u64,
        reactivations: u64,
        pageins: u64,
        pageouts: u64,
        faults: u64,
        cow_faults: u64,
        lookups: u64,
        hits: u64,
        purges: u64,
        purgeable_count: u32,
        speculative_count: u32,
        decompressions: u64,
        compressions: u64,
        swapins: u64,
        swapouts: u64,
        compressor_page_count: u32,
        throttled_count: u32,
        external_page_count: u32,
        internal_page_count: u32,
        total_uncompressed_pages_in_compressor: u64,
    }
    unsafe extern "C" {
        fn mach_host_self() -> u32;
        fn mach_task_self() -> u32;
        fn mach_port_deallocate(task: u32, name: u32) -> i32;
        fn host_page_size(host: u32, page_size: *mut u32) -> i32;
        fn host_statistics64(
            host: u32,
            flavor: i32,
            info: *mut VmStatistics64,
            count: *mut u32,
        ) -> i32;
    }
    const HOST_VM_INFO64: i32 = 4;
    let total = sysctl_u64("hw.memsize");
    let mut page_size = 0u32;
    let host = unsafe { mach_host_self() };
    let page_ok = unsafe { host_page_size(host, &mut page_size) } == 0 && page_size > 0;
    let mut statistics = VmStatistics64::default();
    let mut count = (std::mem::size_of::<VmStatistics64>() / std::mem::size_of::<u32>()) as u32;
    let stats_ok = page_ok
        && unsafe { host_statistics64(host, HOST_VM_INFO64, &mut statistics, &mut count) } == 0;
    unsafe {
        let _ = mach_port_deallocate(mach_task_self(), host);
    }
    let available = stats_ok.then(|| {
        u64::from(statistics.free_count)
            .saturating_add(u64::from(statistics.inactive_count))
            .saturating_add(u64::from(statistics.speculative_count))
            .saturating_mul(u64::from(page_size))
    });
    snapshot.memory.total_bytes = total;
    snapshot.memory.available_bytes = available;
    snapshot.memory.used_bytes = total
        .zip(available)
        .map(|(value, available)| value.saturating_sub(available));
    if total.is_none() || available.is_none() {
        add_warning(snapshot, "physical memory information is unavailable");
    }
}

#[cfg(target_os = "macos")]
fn mac_network(snapshot: &mut SystemSnapshot) {
    #[repr(C)]
    struct IfData {
        generic: [u8; 8],
        mtu: u32,
        metric: u32,
        baudrate: u32,
        ipackets: u32,
        ierrors: u32,
        opackets: u32,
        oerrors: u32,
        collisions: u32,
        ibytes: u32,
        obytes: u32,
    }
    let mut addresses = std::ptr::null_mut();
    if unsafe { libc::getifaddrs(&mut addresses) } != 0 {
        add_warning(snapshot, "network interface information is unavailable");
        return;
    }
    let mut names = BTreeSet::new();
    let mut received = 0u64;
    let mut transmitted = 0u64;
    let mut counters = false;
    let mut current = addresses;
    while !current.is_null() {
        let address = unsafe { (*current).ifa_addr };
        let data = unsafe { (*current).ifa_data };
        if !address.is_null() {
            let name = unsafe { (*current).ifa_name };
            if !name.is_null() {
                let name = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy();
                names.insert(name.into_owned());
            }
            if unsafe { (*address).sa_family } == libc::AF_LINK as u8 && !data.is_null() {
                let stats = unsafe { &*(data.cast::<IfData>()) };
                received = received.saturating_add(u64::from(stats.ibytes));
                transmitted = transmitted.saturating_add(u64::from(stats.obytes));
                counters = true;
            }
        }
        current = unsafe { (*current).ifa_next };
    }
    unsafe { libc::freeifaddrs(addresses) };
    snapshot.network = NetworkSnapshot {
        interface_count: names.len(),
        received_bytes: counters.then_some(received),
        transmitted_bytes: counters.then_some(transmitted),
    };
    if !counters {
        add_warning(snapshot, "macOS interface byte counters are unavailable");
    }
}

#[cfg(target_os = "macos")]
fn mac_processes(snapshot: &mut SystemSnapshot) {
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct ProcTaskInfo {
        virtual_size: u64,
        resident_size: u64,
        total_user: u64,
        total_system: u64,
        threads_user: u64,
        threads_system: u64,
        policy: i32,
        faults: i32,
        pageins: i32,
        cow_faults: i32,
        messages_sent: i32,
        messages_received: i32,
        syscalls_mach: i32,
        syscalls_unix: i32,
        csw: i32,
        threadnum: i32,
        numrunning: i32,
        priority: i32,
    }
    #[link(name = "proc")]
    unsafe extern "C" {
        fn proc_listallpids(buffer: *mut libc::c_void, buffersize: i32) -> i32;
        fn proc_pidinfo(
            pid: i32,
            flavor: u32,
            arg: u64,
            buffer: *mut libc::c_void,
            buffersize: i32,
        ) -> i32;
        fn proc_pidpath(pid: i32, buffer: *mut libc::c_void, buffersize: u32) -> i32;
    }
    const PROC_PIDTASKINFO: u32 = 4;
    let count = unsafe { proc_listallpids(std::ptr::null_mut(), 0) };
    if count <= 0 {
        add_warning(snapshot, "process information is unavailable");
        return;
    }
    let mut pids = vec![0i32; count as usize];
    let copied = unsafe {
        proc_listallpids(
            pids.as_mut_ptr().cast(),
            (pids.len() * std::mem::size_of::<i32>()) as i32,
        )
    };
    if copied <= 0 {
        add_warning(snapshot, "process information is unavailable");
        return;
    }
    let mut rows = Vec::new();
    let mut running = 0usize;
    for pid in pids.into_iter().take(copied as usize) {
        if pid <= 0 {
            continue;
        }
        let mut info = ProcTaskInfo::default();
        let result = unsafe {
            proc_pidinfo(
                pid,
                PROC_PIDTASKINFO,
                0,
                (&mut info as *mut ProcTaskInfo).cast(),
                std::mem::size_of::<ProcTaskInfo>() as i32,
            )
        };
        if result <= 0 {
            continue;
        }
        if info.numrunning > 0 {
            running += 1;
        }
        let mut path = vec![0u8; 1024];
        let path_length = unsafe { proc_pidpath(pid, path.as_mut_ptr().cast(), path.len() as u32) };
        let name = if path_length > 0 {
            let path = String::from_utf8_lossy(&path[..path_length as usize]);
            Path::new(path.as_ref())
                .file_name()
                .and_then(|value| value.to_str())
                .map(sanitize_name)
                .unwrap_or_else(|| pid.to_string())
        } else {
            pid.to_string()
        };
        rows.push(ProcessRow {
            pid: pid as u32,
            name,
            state: None,
            cpu_percent: None,
            memory_bytes: Some(info.resident_size),
        });
    }
    rows.sort_by(|left, right| {
        right
            .memory_bytes
            .unwrap_or(0)
            .cmp(&left.memory_bytes.unwrap_or(0))
            .then_with(|| left.pid.cmp(&right.pid))
    });
    snapshot.processes = ProcessSummary {
        total: rows.len(),
        running,
        top: rows.into_iter().take(MAX_PROCESS_ROWS).collect(),
    };
}

#[cfg(windows)]
fn collect_windows(previous: Option<&SystemSnapshot>) -> SystemSnapshot {
    let mut snapshot = base_snapshot("windows");
    windows_collector::collect(&mut snapshot, previous);
    snapshot
}

#[cfg(windows)]
mod windows_collector {
    use super::*;
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStringExt;

    type Bool = i32;
    type Dword = u32;
    type Handle = *mut c_void;

    const INVALID_HANDLE_VALUE: Handle = -1isize as Handle;
    const TH32CS_SNAPPROCESS: Dword = 0x0000_0002;
    const PROCESS_QUERY_INFORMATION: Dword = 0x0400;
    const PROCESS_VM_READ: Dword = 0x0010;
    const ERROR_INSUFFICIENT_BUFFER: Dword = 122;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct FileTime {
        low: Dword,
        high: Dword,
    }

    #[repr(C)]
    struct MemoryStatus {
        length: Dword,
        memory_load: Dword,
        total_physical: u64,
        available_physical: u64,
        total_page_file: u64,
        available_page_file: u64,
        total_virtual: u64,
        available_virtual: u64,
        available_extended_virtual: u64,
    }

    #[repr(C)]
    struct ProcessEntry {
        size: Dword,
        usage: Dword,
        process_id: Dword,
        heap_id: usize,
        module_id: Dword,
        threads: Dword,
        parent_process_id: Dword,
        base_priority: i32,
        flags: Dword,
        exe_file: [u16; 260],
    }

    #[repr(C)]
    struct ProcessMemoryCounters {
        cb: Dword,
        page_fault_count: Dword,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetSystemTimes(idle: *mut FileTime, kernel: *mut FileTime, user: *mut FileTime) -> Bool;
        fn GlobalMemoryStatusEx(status: *mut MemoryStatus) -> Bool;
        fn GetDiskFreeSpaceExW(
            path: *const u16,
            free_user: *mut u64,
            total: *mut u64,
            free: *mut u64,
        ) -> Bool;
        fn CreateToolhelp32Snapshot(flags: Dword, process_id: Dword) -> Handle;
        fn Process32FirstW(snapshot: Handle, entry: *mut ProcessEntry) -> Bool;
        fn Process32NextW(snapshot: Handle, entry: *mut ProcessEntry) -> Bool;
        fn OpenProcess(access: Dword, inherit: Bool, process_id: Dword) -> Handle;
        fn CloseHandle(handle: Handle) -> Bool;
    }

    #[link(name = "psapi")]
    unsafe extern "system" {
        fn GetProcessMemoryInfo(
            process: Handle,
            counters: *mut ProcessMemoryCounters,
            size: Dword,
        ) -> Bool;
    }

    #[link(name = "iphlpapi")]
    unsafe extern "system" {
        fn GetIfTable(table: *mut u8, size: *mut Dword, order: Bool) -> Dword;
    }

    pub(super) fn collect(snapshot: &mut SystemSnapshot, previous: Option<&SystemSnapshot>) {
        collect_cpu(snapshot, previous);
        collect_memory(snapshot);
        collect_disks(snapshot);
        collect_network(snapshot);
        collect_processes(snapshot);
    }

    fn file_time(value: FileTime) -> u64 {
        (u64::from(value.high) << 32) | u64::from(value.low)
    }

    fn collect_cpu(snapshot: &mut SystemSnapshot, previous: Option<&SystemSnapshot>) {
        let mut idle = FileTime::default();
        let mut kernel = FileTime::default();
        let mut user = FileTime::default();
        if unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) } == 0 {
            add_warning(snapshot, "Windows CPU counters are unavailable");
            return;
        }
        let idle = file_time(idle);
        let total = file_time(kernel).saturating_add(file_time(user));
        snapshot.cpu_total = Some(total);
        snapshot.cpu_idle = Some(idle);
        snapshot.cpu.usage_percent = cpu_usage(previous, Some(total), Some(idle));
    }

    fn collect_memory(snapshot: &mut SystemSnapshot) {
        let mut status = MemoryStatus {
            length: std::mem::size_of::<MemoryStatus>() as Dword,
            memory_load: 0,
            total_physical: 0,
            available_physical: 0,
            total_page_file: 0,
            available_page_file: 0,
            total_virtual: 0,
            available_virtual: 0,
            available_extended_virtual: 0,
        };
        if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 {
            add_warning(snapshot, "Windows memory counters are unavailable");
            return;
        }
        snapshot.memory.total_bytes = Some(status.total_physical);
        snapshot.memory.available_bytes = Some(status.available_physical);
        snapshot.memory.used_bytes = Some(
            status
                .total_physical
                .saturating_sub(status.available_physical),
        );
        snapshot.memory.swap_total_bytes = Some(status.total_page_file);
        snapshot.memory.swap_used_bytes = Some(
            status
                .total_page_file
                .saturating_sub(status.available_page_file),
        );
    }

    fn collect_disks(snapshot: &mut SystemSnapshot) {
        for letter in b'A'..=b'Z' {
            let path = [letter as u16, b':' as u16, b'\\' as u16, 0];
            let mut free_user = 0u64;
            let mut total = 0u64;
            let mut free = 0u64;
            if unsafe { GetDiskFreeSpaceExW(path.as_ptr(), &mut free_user, &mut total, &mut free) }
                != 0
            {
                let mount = format!("{}:\\", letter as char);
                snapshot.disks.push(DiskSnapshot {
                    mount,
                    total_bytes: Some(total),
                    used_bytes: Some(total.saturating_sub(free)),
                    available_bytes: Some(free),
                });
            }
        }
        if snapshot.disks.is_empty() {
            add_warning(snapshot, "Windows disk counters are unavailable");
        }
    }

    fn collect_network(snapshot: &mut SystemSnapshot) {
        let mut size = 0u32;
        let result = unsafe { GetIfTable(std::ptr::null_mut(), &mut size, 0) };
        if result != ERROR_INSUFFICIENT_BUFFER || size < 4 {
            add_warning(snapshot, "Windows network counters are unavailable");
            return;
        }
        let mut buffer = vec![0u8; size as usize];
        let result = unsafe { GetIfTable(buffer.as_mut_ptr(), &mut size, 0) };
        if result != 0 {
            add_warning(snapshot, "Windows network counters are unavailable");
            return;
        }
        let count = u32::from_ne_bytes(buffer[..4].try_into().unwrap_or([0; 4]));
        snapshot.network.interface_count = count as usize;
        add_warning(
            snapshot,
            "Windows portable collector reports interface count; byte counters are deferred",
        );
    }

    fn collect_processes(snapshot: &mut SystemSnapshot) {
        let handle = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if handle == INVALID_HANDLE_VALUE {
            add_warning(snapshot, "Windows process information is unavailable");
            return;
        }
        let mut entry = ProcessEntry {
            size: std::mem::size_of::<ProcessEntry>() as Dword,
            usage: 0,
            process_id: 0,
            heap_id: 0,
            module_id: 0,
            threads: 0,
            parent_process_id: 0,
            base_priority: 0,
            flags: 0,
            exe_file: [0; 260],
        };
        let mut rows = Vec::new();
        let mut running = 0usize;
        let mut total = 0usize;
        let mut has_entry = unsafe { Process32FirstW(handle, &mut entry) } != 0;
        while has_entry {
            total += 1;
            let name_end = entry
                .exe_file
                .iter()
                .position(|value| *value == 0)
                .unwrap_or(entry.exe_file.len());
            let name = std::ffi::OsString::from_wide(&entry.exe_file[..name_end])
                .to_string_lossy()
                .into_owned();
            let process = unsafe {
                OpenProcess(
                    PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
                    0,
                    entry.process_id,
                )
            };
            let memory_bytes = if process.is_null() {
                None
            } else {
                let mut counters = ProcessMemoryCounters {
                    cb: std::mem::size_of::<ProcessMemoryCounters>() as Dword,
                    page_fault_count: 0,
                    peak_working_set_size: 0,
                    working_set_size: 0,
                    quota_peak_paged_pool_usage: 0,
                    quota_paged_pool_usage: 0,
                    quota_peak_non_paged_pool_usage: 0,
                    quota_non_paged_pool_usage: 0,
                    pagefile_usage: 0,
                    peak_pagefile_usage: 0,
                };
                let result = unsafe {
                    GetProcessMemoryInfo(
                        process,
                        &mut counters,
                        std::mem::size_of::<ProcessMemoryCounters>() as Dword,
                    )
                };
                unsafe { CloseHandle(process) };
                (result != 0).then_some(counters.working_set_size as u64)
            };
            rows.push(ProcessRow {
                pid: entry.process_id,
                name: sanitize_name(&name),
                state: None,
                cpu_percent: None,
                memory_bytes,
            });
            has_entry = unsafe { Process32NextW(handle, &mut entry) } != 0;
        }
        unsafe { CloseHandle(handle) };
        rows.sort_by(|left, right| {
            right
                .memory_bytes
                .unwrap_or(0)
                .cmp(&left.memory_bytes.unwrap_or(0))
                .then_with(|| left.pid.cmp(&right.pid))
        });
        snapshot.processes = ProcessSummary {
            total,
            running,
            top: rows.into_iter().take(MAX_PROCESS_ROWS).collect(),
        };
    }
}

fn sanitize_name(value: &str) -> String {
    value
        .chars()
        .filter(|value| !value.is_control())
        .take(64)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_is_read_only_and_json_has_a_stable_contract() {
        let snapshot = collect_snapshot(None);
        assert_eq!(snapshot.schema_version, SCHEMA_VERSION);
        assert_eq!(snapshot.contract, SYSTEM_MONITOR_CONTRACT);
        assert!(snapshot.read_only);
        assert!(!snapshot.writes_attempted);
        let json = render_json(&snapshot).expect("snapshot json");
        assert!(json.contains("system_monitor_snapshot"));
        assert!(!json.contains("cpu_total"));
    }

    #[test]
    fn byte_and_duration_rendering_is_bounded_and_human() {
        assert_eq!(format_bytes(Some(0)), "0 B");
        assert_eq!(format_bytes(Some(1024)), "1.0 KiB");
        assert_eq!(format_duration(86_461), "1d 00h 01m");
        assert_eq!(format_duration(3_661), "01h 01m 01s");
    }

    #[test]
    fn load_values_reject_invalid_samples() {
        let values = load_from_values([0.5, f64::NAN, -1.0]);
        assert_eq!(values, [Some(500), None, None]);
    }
}
