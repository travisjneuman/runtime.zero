#!/usr/bin/env python3
"""Exercise a final rz0 artifact through bounded macOS/POSIX pseudo-terminals."""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import platform
import pty
import select
import signal
import stat
import struct
import subprocess
import sys
import termios
import time

MAX_CAPTURE_BYTES = 1024 * 1024
CASES = (
    ("xterm-256color-standard", "xterm-256color", 80, 24, None),
    ("xterm-compact-resize", "xterm", 40, 12, (100, 30)),
    ("screen-wide", "screen", 120, 40, None),
    ("vt100-minimum", "vt100", 40, 12, None),
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--target", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--arch")
    return parser.parse_args()


def set_size(descriptor: int, columns: int, rows: int) -> None:
    dimensions = struct.pack("HHHH", rows, columns, 0, 0)
    fcntl.ioctl(descriptor, termios.TIOCSWINSZ, dimensions)


def command_for(binary: Path, architecture: str | None) -> list[str]:
    command = [str(binary), "--tui"]
    if architecture:
        if platform.system() != "Darwin" or architecture not in ("arm64", "x86_64"):
            raise ValueError("--arch accepts arm64 or x86_64 only on macOS")
        command = ["/usr/bin/arch", f"-{architecture}", *command]
    return command


def run_case(binary: Path, architecture: str | None, terminal: str, columns: int, rows: int, resize: tuple[int, int] | None) -> dict:
    master, slave = pty.openpty()
    set_size(slave, columns, rows)
    environment = {
        "LANG": "C.UTF-8" if platform.system() != "Darwin" else "en_US.UTF-8",
        "LC_ALL": "C.UTF-8" if platform.system() != "Darwin" else "en_US.UTF-8",
        "NO_COLOR": "1",
        "TERM": terminal,
    }
    child = subprocess.Popen(
        command_for(binary, architecture),
        stdin=slave,
        stdout=slave,
        stderr=slave,
        env=environment,
        close_fds=True,
        start_new_session=True,
    )
    os.close(slave)
    capture = bytearray()
    truncated = False
    resized = False
    quit_sent = False
    started = time.monotonic()
    deadline = started + 4.0
    try:
        while time.monotonic() < deadline:
            elapsed = time.monotonic() - started
            if resize and not resized and elapsed >= 0.15:
                set_size(master, resize[0], resize[1])
                os.kill(child.pid, signal.SIGWINCH)
                resized = True
            if not quit_sent and elapsed >= 0.30:
                os.write(master, b"q")
                quit_sent = True
            readable, _, _ = select.select([master], [], [], 0.05)
            if readable:
                try:
                    block = os.read(master, 8192)
                except OSError:
                    block = b""
                remaining = MAX_CAPTURE_BYTES - len(capture)
                capture.extend(block[:remaining])
                truncated |= len(block) > remaining
            if child.poll() is not None:
                while True:
                    readable, _, _ = select.select([master], [], [], 0)
                    if not readable:
                        break
                    try:
                        block = os.read(master, 8192)
                    except OSError:
                        break
                    if not block:
                        break
                    remaining = MAX_CAPTURE_BYTES - len(capture)
                    capture.extend(block[:remaining])
                    truncated |= len(block) > remaining
                break
        if child.poll() is None:
            os.killpg(child.pid, signal.SIGKILL)
            child.wait(timeout=1)
            raise ValueError("TUI did not exit after bounded q input")
    finally:
        os.close(master)
    output = bytes(capture)
    entered = b"\x1b[?1049h" in output
    left = b"\x1b[?1049l" in output
    passed = (
        child.returncode == 0
        and quit_sent
        and not truncated
        and entered
        and left
        and b"q\r\n" not in output
    )
    return {
        "terminal": terminal,
        "initial_columns": columns,
        "initial_rows": rows,
        "resized_columns": resize[0] if resize else None,
        "resized_rows": resize[1] if resize else None,
        "alternate_screen_entered": entered,
        "alternate_screen_left": left,
        "quit_input_echoed": b"q\r\n" in output,
        "exit_code": child.returncode,
        "captured_bytes": len(output),
        "capture_sha256": hashlib.sha256(output).hexdigest(),
        "capture_truncated": truncated,
        "passed": passed,
    }


def write_new(path: Path, data: bytes) -> None:
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o644)
    with os.fdopen(descriptor, "wb") as output:
        output.write(data)
        output.flush()
        os.fsync(output.fileno())


def main() -> int:
    args = parse_args()
    if len(args.source_commit) != 40 or any(character not in "0123456789abcdef" for character in args.source_commit):
        raise ValueError("source commit must be a full lowercase Git SHA-1")
    metadata = args.binary.lstat()
    if args.binary.is_symlink() or not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
        raise ValueError("binary must be a single-link regular file")
    binary = args.binary.read_bytes()
    if not binary or len(binary) > 64 * 1024 * 1024 or len(binary) != metadata.st_size:
        raise ValueError("binary is empty, oversized, or changed while reading")
    artifact_sha256 = hashlib.sha256(binary).hexdigest()

    cases = []
    for case_id, terminal, columns, rows, resize in CASES:
        result = run_case(args.binary, args.arch, terminal, columns, rows, resize)
        result["case_id"] = case_id
        cases.append(result)
    passed = all(case["passed"] for case in cases)
    architecture_suffix = f"-{args.arch}" if args.arch else ""
    evidence = {
        "schema_version": 1,
        "contract": "final_artifact_terminal_smoke",
        "evidence_id": f"terminal:{args.target}{architecture_suffix}-{artifact_sha256[:12]}",
        "target": args.target,
        "source_commit": args.source_commit,
        "artifact_sha256": artifact_sha256,
        "read_only": True,
        "writes_attempted": False,
        "decision": "pass" if passed else "blocked",
        "release_authorized": False,
        "cases": cases,
    }
    data = (json.dumps(evidence, indent=2, sort_keys=True) + "\n").encode("utf-8")
    if len(data) > 64 * 1024:
        raise ValueError("terminal evidence exceeds its document ceiling")
    write_new(args.output, data)
    print(json.dumps({
        "artifact_sha256": artifact_sha256,
        "case_count": len(cases),
        "decision": evidence["decision"],
    }, sort_keys=True))
    return 0 if passed else 3


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"smoke_terminal_artifact: {error}", file=sys.stderr)
        raise SystemExit(2) from error
