#!/usr/bin/env python3
"""Measure bounded runtime.zero final-artifact command performance on POSIX hosts."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import resource
import stat
import subprocess
import sys
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


def invoke(binary: Path, arguments: list[str], architecture: str | None) -> tuple[int, int, int]:
    command = [str(binary), *arguments]
    if architecture:
        if platform.system() != "Darwin" or architecture not in ("arm64", "x86_64"):
            raise ValueError("--arch accepts arm64 or x86_64 only on macOS")
        command = ["/usr/bin/arch", f"-{architecture}", *command]
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
        for _ in range(args.warmups):
            invoke(args.binary, arguments, args.arch)
        times = []
        maximum_stdout = 0
        maximum_stderr = 0
        maximum_resident = resident_bytes()
        for _ in range(args.samples):
            elapsed, stdout_bytes, stderr_bytes = invoke(args.binary, arguments, args.arch)
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
            "successful_samples": args.samples,
        }
        within_budget &= (
            measurement["p95_wall_time_us"] <= BUDGET["p95_wall_time_us"]
            and measurement["maximum_wall_time_us"] <= BUDGET["maximum_wall_time_us"]
            and measurement["maximum_resident_bytes"] <= BUDGET["maximum_resident_bytes"]
            and maximum_stdout + maximum_stderr <= BUDGET["maximum_output_bytes"]
        )
        measurements.append(measurement)

    architecture_suffix = f"-{args.arch}" if args.arch else ""
    evidence = {
        "schema_version": 2,
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
