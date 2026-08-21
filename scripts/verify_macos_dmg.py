#!/usr/bin/env python3
"""Verify an unsigned runtime.zero macOS DMG without publishing or installing it."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import sys

MAX_DMG_BYTES = 128 * 1024 * 1024
MAX_FILE_BYTES = 64 * 1024 * 1024
MAX_OUTPUT_BYTES = 2 * 1024 * 1024
EXPECTED_FILES = {
    "LICENSE",
    "README.md",
    "SAFETY.md",
    "SECURITY.md",
    "SBOM.spdx.json",
    "THIRD-PARTY-NOTICES.txt",
    "artifact-manifest.json",
    "dmg-manifest.json",
    "rz0",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dmg", required=True, type=Path)
    parser.add_argument("--checksum", required=True, type=Path)
    parser.add_argument("--mountpoint", required=True, type=Path)
    parser.add_argument("--target", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--source-zip", type=Path)
    parser.add_argument("--expected-dmg-sha256")
    return parser.parse_args()


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_regular(path: Path, maximum: int) -> bytes:
    metadata = path.lstat()
    if path.is_symlink() or not stat.S_ISREG(metadata.st_mode):
        raise ValueError(f"unsafe non-regular file: {path}")
    if metadata.st_size == 0 or metadata.st_size > maximum:
        raise ValueError(f"empty or oversized file: {path.name}")
    data = path.read_bytes()
    if len(data) != metadata.st_size:
        raise ValueError(f"file changed while reading: {path.name}")
    return data


def validate_sha256_file(checksum: Path, archive_name: str, actual: str) -> None:
    fields = read_regular(checksum, 1024).decode("ascii").strip().split()
    if fields != [actual, archive_name]:
        raise ValueError("DMG checksum file does not bind the exact image bytes")


def run_read_only(binary: Path, arguments: list[str]) -> None:
    environment = {
        "LANG": "C",
        "LC_ALL": "C",
        "NO_COLOR": "1",
        "TERM": "dumb",
        "PATH": "/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin:/usr/local/bin",
    }
    completed = subprocess.run(
        [str(binary), *arguments],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=environment,
        timeout=5,
        check=False,
    )
    if completed.returncode != 0:
        raise ValueError(f"DMG binary failed for {' '.join(arguments)}: {completed.returncode}")
    if len(completed.stdout) + len(completed.stderr) > MAX_OUTPUT_BYTES:
        raise ValueError(f"DMG binary output exceeded {MAX_OUTPUT_BYTES} bytes")


def content_digest(files: dict[str, bytes]) -> str:
    content = hashlib.sha256()
    content.update(b"runtime.zero.macos-dmg-content.v1\0")
    for name in sorted(files):
        data = files[name]
        name_bytes = name.encode("utf-8")
        content.update(len(name_bytes).to_bytes(8, "big"))
        content.update(name_bytes)
        content.update(len(data).to_bytes(8, "big"))
        content.update(data)
    return content.hexdigest()


def verify(args: argparse.Namespace) -> dict[str, object]:
    if len(args.source_commit) != 40 or any(
        character not in "0123456789abcdef" for character in args.source_commit
    ):
        raise ValueError("source commit must be a full lowercase Git SHA-1")
    dmg = args.dmg.resolve(strict=True)
    dmg_bytes = read_regular(dmg, MAX_DMG_BYTES)
    dmg_sha256 = digest(dmg_bytes)
    if args.expected_dmg_sha256 and args.expected_dmg_sha256 != dmg_sha256:
        raise ValueError("DMG SHA-256 does not match the expected digest")
    validate_sha256_file(args.checksum.resolve(strict=True), dmg.name, dmg_sha256)

    mountpoint = args.mountpoint.resolve()
    metadata = mountpoint.lstat()
    if mountpoint.is_symlink() or not stat.S_ISDIR(metadata.st_mode):
        raise ValueError("mountpoint must be an existing direct directory")
    if any(mountpoint.iterdir()):
        raise ValueError("mountpoint must be empty before read-only attachment")

    attached = False
    try:
        subprocess.run(
            [
                "hdiutil",
                "attach",
                "-readonly",
                "-nobrowse",
                "-mountpoint",
                str(mountpoint),
                str(dmg),
            ],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=30,
        )
        attached = True
        files = {
            path.name: read_regular(path, MAX_FILE_BYTES)
            for path in mountpoint.iterdir()
        }
        if set(files) != EXPECTED_FILES:
            raise ValueError("DMG content does not match the exact nine-file contract")
        artifact_manifest = json.loads(files["artifact-manifest.json"])
        if (
            artifact_manifest.get("contract") != "release_artifact_manifest"
            or artifact_manifest.get("target") != args.target
            or artifact_manifest.get("source_commit") != args.source_commit
            or artifact_manifest.get("signature_posture") != "unsigned"
            or artifact_manifest.get("notarized") is not False
            or artifact_manifest.get("binary", {}).get("sha256") != digest(files["rz0"])
            or artifact_manifest.get("binary", {}).get("size_bytes") != len(files["rz0"])
        ):
            raise ValueError("portable artifact manifest is inconsistent with the DMG")

        dmg_manifest = json.loads(files["dmg-manifest.json"])
        if (
            dmg_manifest.get("contract") != "macos_dmg_manifest"
            or dmg_manifest.get("target") != args.target
            or dmg_manifest.get("source_commit") != args.source_commit
            or dmg_manifest.get("source_portable_zip_sha256") is None
            or dmg_manifest.get("content_sha256")
            != content_digest({name: data for name, data in files.items() if name != "dmg-manifest.json"})
            or dmg_manifest.get("signature_posture") != "unsigned"
            or dmg_manifest.get("notarized") is not False
            or dmg_manifest.get("container_reproducible") is not False
        ):
            raise ValueError("DMG manifest is inconsistent with mounted content")
        if args.source_zip:
            source_zip = read_regular(args.source_zip.resolve(strict=True), MAX_DMG_BYTES)
            if digest(source_zip) != dmg_manifest["source_portable_zip_sha256"]:
                raise ValueError("DMG manifest does not bind the supplied source ZIP")

        binary = mountpoint / "rz0"
        run_read_only(binary, ["--version"])
        run_read_only(binary, ["doctor"])
        run_read_only(binary, ["scan", "--dry-run"])
        return {
            "contract": "macos_dmg_verification",
            "dmg_sha256": dmg_sha256,
            "source_commit": args.source_commit,
            "target": args.target,
            "entry_count": len(files),
            "read_only": True,
            "writes_attempted": False,
            "release_authorized": False,
            "decision": "pass",
        }
    finally:
        if attached:
            subprocess.run(
                ["hdiutil", "detach", str(mountpoint)],
                check=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=30,
            )


def main() -> int:
    result = verify(parse_args())
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError, subprocess.SubprocessError) as error:
        print(f"verify_macos_dmg: {error}", file=sys.stderr)
        raise SystemExit(2) from error
