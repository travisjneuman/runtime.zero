#!/usr/bin/env python3
"""Measure bounded runtime.zero final-artifact command performance on POSIX hosts."""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import pty
import resource
import select
import signal
import stat
import struct
import subprocess
import sys
import termios
import time

MIN_SAMPLES = 10
MAX_SAMPLES = 100
BUDGET = {
    "p95_wall_time_us": 1_000_000,
    "maximum_wall_time_us": 2_000_000,
    "maximum_resident_bytes": 64 * 1024 * 1024,
    "maximum_output_bytes": 2 * 1024 * 1024,
}
OPERATIONS = (
    ("version", ["--version"]),
    ("doctor_text", ["doctor"]),
    ("doctor_json", ["doctor", "--format", "json"]),
    ("core_scan_text", ["scan", "--dry-run"]),
    ("core_scan_json", ["scan", "--dry-run", "--format", "json"]),
    ("apps_json", ["apps", "--format", "json"]),
    ("monitor_json", ["monitor", "--format", "json"]),
    ("report_json", ["report", "--format", "json"]),
    ("dashboard_json", ["--json"]),
)
TUI_OPERATIONS = ("tui_startup", "tui_refresh")
MAX_CAPTURE_BYTES = 1024 * 1024
TUI_TIMEOUT_SECONDS = 5.0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--target", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--samples", type=int, default=25)
    parser.add_argument("--warmups", type=int, default=3)
    parser.add_argument("--arch")
    return parser.parse_args()


def percentile(values: list[int], fraction: float) -> int:
    ordered = sorted(values)
    index = max(0, math.ceil(len(ordered) * fraction) - 1)
    return ordered[index]


def resident_bytes() -> int:
    resident = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
    return int(resident if platform.system() == "Darwin" else resident * 1024)


def set_pty_size(descriptor: int, columns: int, rows: int) -> None:
    dimensions = struct.pack("HHHH", rows, columns, 0, 0)
    fcntl.ioctl(descriptor, termios.TIOCSWINSZ, dimensions)


def command_for(binary: Path, architecture: str | None, arguments: list[str]) -> list[str]:
    command = [str(binary), *arguments]
    if architecture:
        if platform.system() != "Darwin" or architecture not in ("arm64", "x86_64"):
            raise ValueError("--arch accepts arm64 or x86_64 only on macOS")
        command = ["/usr/bin/arch", f"-{architecture}", *command]
    return command


def pty_read(master: int, capture: bytearray, timeout: float) -> None:
    readable, _, _ = select.select([master], [], [], timeout)
    if not readable:
        return
    try:
        block = os.read(master, 8192)
    except OSError:
        return
    remaining = MAX_CAPTURE_BYTES - len(capture)
    capture.extend(block[:remaining])


def stop_child(child: subprocess.Popen[bytes]) -> None:
    if child.poll() is not None:
        return
    try:
        os.killpg(child.pid, signal.SIGKILL)
    except (OSError, PermissionError):
        pass
    try:
        child.wait(timeout=1)
    except subprocess.TimeoutExpired:
        pass


def run_tui_measurement(binary: Path, architecture: str | None, operation: str) -> tuple[int, int, int]:
    if operation not in TUI_OPERATIONS:
        raise ValueError(f"unsupported TUI performance operation: {operation}")
    master, slave = pty.openpty()
    set_pty_size(slave, 80, 24)
    environment = {
        "LANG": "C.UTF-8" if platform.system() != "Darwin" else "en_US.UTF-8",
        "LC_ALL": "C.UTF-8" if platform.system() != "Darwin" else "en_US.UTF-8",
        "NO_COLOR": "1",
        "TERM": "xterm-256color",
    }
    home = os.environ.get("HOME")
    if home and os.path.isabs(home) and len(os.fsencode(home)) <= 4096:
        environment["HOME"] = home
    child: subprocess.Popen[bytes] | None = None
    capture = bytearray()
    try:
        child = subprocess.Popen(
            command_for(binary, architecture, ["--tui"]),
            stdin=slave,
            stdout=slave,
            stderr=slave,
            env=environment,
            close_fds=True,
            start_new_session=True,
        )
        os.close(slave)
        slave = -1
        started = time.perf_counter_ns()
        first_deadline = time.monotonic() + TUI_TIMEOUT_SECONDS
        first_frame = False
        while time.monotonic() < first_deadline:
            pty_read(master, capture, 0.02)
            output = bytes(capture)
            if b"\x1b[?1049h" in output and b"runtime.zero" in output:
                first_frame = True
                break
            if child.poll() is not None:
                break
        if not first_frame:
            raise ValueError(f"TUI {operation} did not render its first frame")

        if operation == "tui_startup":
            elapsed_us = (time.perf_counter_ns() - started + 999) // 1000
        else:
            refresh_baseline = len(capture)
            refresh_started = time.perf_counter_ns()
            os.write(master, b"r")
            refresh_deadline = time.monotonic() + TUI_TIMEOUT_SECONDS
            refreshed = False
            while time.monotonic() < refresh_deadline:
                pty_read(master, capture, 0.02)
                if b"refreshing" in bytes(capture)[refresh_baseline:]:
                    refreshed = True
                    break
                if child.poll() is not None:
                    break
            if not refreshed:
                raise ValueError("TUI refresh did not render its explicit refreshing state")
            elapsed_us = (time.perf_counter_ns() - refresh_started + 999) // 1000

        os.write(master, b"q")
        quit_deadline = time.monotonic() + 2.0
        while child.poll() is None and time.monotonic() < quit_deadline:
            pty_read(master, capture, 0.02)
        for _ in range(5):
            pty_read(master, capture, 0)
        if child.poll() is None:
            raise ValueError("TUI did not exit after bounded q input")
        if child.returncode != 0:
            raise ValueError(f"TUI exited with {child.returncode}")
        output = bytes(capture)
        if b"\x1b[?1049h" not in output or b"\x1b[?1049l" not in output:
            raise ValueError("TUI did not enter and leave the alternate screen")
        if b"q\r\n" in output:
            raise ValueError("TUI quit input was echoed")
        if len(output) >= MAX_CAPTURE_BYTES:
            raise ValueError("TUI output exceeded its capture ceiling")
        return elapsed_us, len(output), 0
    finally:
        if child is not None:
            stop_child(child)
        if slave >= 0:
            os.close(slave)
        os.close(master)


def invoke(binary: Path, arguments: list[str], architecture: str | None) -> tuple[int, int, int]:
    command = command_for(binary, architecture, arguments)
    environment = {
        "LANG": "C",
        "LC_ALL": "C",
        "NO_COLOR": "1",
        "TERM": "dumb",
        "PATH": (
            "/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin:/usr/local/bin"
            if platform.system() == "Darwin"
            else "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
        ),
    }
    home = os.environ.get("HOME")
    if home and os.path.isabs(home) and len(os.fsencode(home)) <= 4096:
        environment["HOME"] = home
    started = time.perf_counter_ns()
    completed = subprocess.run(
        command,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=environment,
        timeout=3,
        check=False,
    )
    elapsed_us = (time.perf_counter_ns() - started + 999) // 1000
    if completed.returncode != 0:
        raise ValueError(f"final artifact command failed with exit {completed.returncode}")
    return elapsed_us, len(completed.stdout), len(completed.stderr)


def measure_operation(operation: str, samples: int, warmups: int, runner) -> tuple[dict, bool]:
    for _ in range(warmups):
        runner()
    times = []
    maximum_stdout = 0
    maximum_stderr = 0
    maximum_resident = resident_bytes()
    for _ in range(samples):
        elapsed, stdout_bytes, stderr_bytes = runner()
        times.append(elapsed)
        maximum_stdout = max(maximum_stdout, stdout_bytes)
        maximum_stderr = max(maximum_stderr, stderr_bytes)
        maximum_resident = max(maximum_resident, resident_bytes())
    measurement = {
        "operation": operation,
        "p50_wall_time_us": percentile(times, 0.50),
        "p95_wall_time_us": percentile(times, 0.95),
        "maximum_wall_time_us": max(times),
        "maximum_resident_bytes": maximum_resident,
        "maximum_stdout_bytes": maximum_stdout,
        "maximum_stderr_bytes": maximum_stderr,
        "successful_samples": samples,
    }
    within_budget = (
        measurement["p95_wall_time_us"] <= BUDGET["p95_wall_time_us"]
        and measurement["maximum_wall_time_us"] <= BUDGET["maximum_wall_time_us"]
        and measurement["maximum_resident_bytes"] <= BUDGET["maximum_resident_bytes"]
        and maximum_stdout + maximum_stderr <= BUDGET["maximum_output_bytes"]
    )
    return measurement, within_budget


def write_new(path: Path, data: bytes) -> None:
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o644)
    try:
        with os.fdopen(descriptor, "wb") as output:
            output.write(data)
            output.flush()
            os.fsync(output.fileno())
    except BaseException:
        try:
            path.unlink()
        except FileNotFoundError:
            pass
        raise


def main() -> int:
    args = parse_args()
    if not MIN_SAMPLES <= args.samples <= MAX_SAMPLES:
        raise ValueError(f"samples must be {MIN_SAMPLES}..={MAX_SAMPLES}")
    if not 0 <= args.warmups <= 10:
        raise ValueError("warmups must be 0..=10")
    if len(args.source_commit) != 40 or any(character not in "0123456789abcdef" for character in args.source_commit):
        raise ValueError("source commit must be a full lowercase Git SHA-1")
    metadata = args.binary.lstat()
    if args.binary.is_symlink() or not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
        raise ValueError("binary must be a single-link regular file")
    binary = args.binary.read_bytes()
    if not binary or len(binary) > 64 * 1024 * 1024:
        raise ValueError("binary is empty or oversized")
    artifact_sha256 = hashlib.sha256(binary).hexdigest()

    measurements = []
    within_budget = True
    for operation, arguments in OPERATIONS:
        measurement, operation_within_budget = measure_operation(
            operation,
            args.samples,
            args.warmups,
            lambda: invoke(args.binary, arguments, args.arch),
        )
        measurements.append(measurement)
        within_budget &= operation_within_budget
    for operation in TUI_OPERATIONS:
        measurement, operation_within_budget = measure_operation(
            operation,
            args.samples,
            args.warmups,
            lambda operation=operation: run_tui_measurement(args.binary, args.arch, operation),
        )
        measurements.append(measurement)
        within_budget &= operation_within_budget

    architecture_suffix = f"-{args.arch}" if args.arch else ""
    evidence = {
        "schema_version": 3,
        "contract": "final_artifact_performance",
        "evidence_id": f"perf:{args.target}{architecture_suffix}-{artifact_sha256[:12]}",
        "target": args.target,
        "source_commit": args.source_commit,
        "artifact_sha256": artifact_sha256,
        "sample_count": args.samples,
        "decision": "pass" if within_budget else "blocked",
        "release_authorized": False,
        "budget": BUDGET,
        "operations": measurements,
    }
    data = (json.dumps(evidence, indent=2, sort_keys=True) + "\n").encode("utf-8")
    if len(data) > 64 * 1024:
        raise ValueError("performance evidence exceeds its document ceiling")
    write_new(args.output, data)
    print(json.dumps({
        "artifact_sha256": artifact_sha256,
        "decision": evidence["decision"],
        "evidence_id": evidence["evidence_id"],
    }, sort_keys=True))
    return 0 if within_budget else 3


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"benchmark_final_artifact: {error}", file=sys.stderr)
        raise SystemExit(2) from error
