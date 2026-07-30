#!/usr/bin/env python3
"""Verify a canonical portable ZIP and prepare deterministic macOS DMG contents."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import stat
import sys
import zipfile

MAX_FILE_BYTES = 64 * 1024 * 1024
MAX_TOTAL_BYTES = 70 * 1024 * 1024
FIXED_TIME = 1_577_836_800
PUBLIC_FILES = {"README.md", "LICENSE", "SAFETY.md", "SECURITY.md"}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", required=True, type=Path)
    parser.add_argument("--checksum", required=True, type=Path)
    parser.add_argument("--staging", required=True, type=Path)
    parser.add_argument("--target", required=True, choices=("aarch64-apple-darwin", "x86_64-apple-darwin"))
    parser.add_argument("--version", required=True)
    parser.add_argument("--source-commit", required=True)
    return parser.parse_args()


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_regular(path: Path, maximum: int) -> bytes:
    metadata = path.lstat()
    if path.is_symlink() or not stat.S_ISREG(metadata.st_mode) or metadata.st_size > maximum:
        raise ValueError(f"unsafe or oversized input: {path.name}")
    data = path.read_bytes()
    if len(data) != metadata.st_size:
        raise ValueError(f"input size changed while reading: {path.name}")
    return data


def validate_checksum(checksum: bytes, archive_name: str, archive_sha256: str) -> None:
    expected = f"{archive_sha256}  {archive_name}\n".encode("ascii")
    if checksum != expected:
        raise ValueError("portable ZIP checksum does not match exact archive bytes")


def write_new(path: Path, data: bytes, mode: int) -> None:
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, mode)
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
    os.chmod(path, mode)
    os.utime(path, (FIXED_TIME, FIXED_TIME), follow_symlinks=False)


def main() -> int:
    args = parse_args()
    archive_bytes = read_regular(args.archive, MAX_TOTAL_BYTES)
    archive_sha256 = digest(archive_bytes)
    checksum = read_regular(args.checksum, 1024)
    validate_checksum(checksum, args.archive.name, archive_sha256)

    prefix = f"runtime-zero-{args.version}-{args.target}/"
    expected = PUBLIC_FILES | {"rz0", "artifact-manifest.json"}
    files: dict[str, bytes] = {}
    with zipfile.ZipFile(args.archive) as package:
        infos = package.infolist()
        if len(infos) != len(expected) or len({info.filename for info in infos}) != len(infos):
            raise ValueError("portable ZIP has missing, duplicate, or extra entries")
        total = 0
        for info in infos:
            path = PurePosixPath(info.filename)
            if not info.filename.startswith(prefix) or path.is_absolute() or ".." in path.parts:
                raise ValueError("portable ZIP contains an unsafe path")
            relative = info.filename[len(prefix):]
            if relative not in expected or "/" in relative:
                raise ValueError("portable ZIP contains an unexpected entry")
            mode = (info.external_attr >> 16) & 0xFFFF
            if not stat.S_ISREG(mode) or info.file_size > MAX_FILE_BYTES:
                raise ValueError("portable ZIP contains an unsafe file type or size")
            total += info.file_size
            if total > MAX_TOTAL_BYTES:
                raise ValueError("portable ZIP exceeds the expanded byte ceiling")
            data = package.read(info)
            if len(data) != info.file_size:
                raise ValueError("portable ZIP entry size changed during extraction")
            files[relative] = data
    if set(files) != expected:
        raise ValueError("portable ZIP does not contain the exact canonical file set")

    manifest = json.loads(files["artifact-manifest.json"])
    binary = files["rz0"]
    if (
        manifest.get("schema_version") != 1
        or manifest.get("contract") != "release_artifact_manifest"
        or manifest.get("version") != args.version
        or manifest.get("target") != args.target
        or manifest.get("source_commit") != args.source_commit
        or manifest.get("distribution") != "github_portable_zip"
        or manifest.get("signature_posture") != "unsigned"
        or manifest.get("notarized") is not False
        or manifest.get("binary", {}).get("sha256") != digest(binary)
        or manifest.get("binary", {}).get("size_bytes") != len(binary)
    ):
        raise ValueError("portable ZIP manifest is inconsistent with DMG inputs")

    staging = args.staging
    staging.mkdir(mode=0o700)
    if staging.is_symlink() or not staging.is_dir() or any(staging.iterdir()):
        raise ValueError("staging must be a new empty direct directory")
    for name in sorted(files):
        write_new(staging / name, files[name], 0o755 if name == "rz0" else 0o644)

    content = hashlib.sha256()
    content.update(b"runtime.zero.macos-dmg-content.v1\0")
    for name in sorted(files):
        data = files[name]
        content.update(len(name).to_bytes(8, "big"))
        content.update(name.encode("utf-8"))
        content.update(len(data).to_bytes(8, "big"))
        content.update(data)
    dmg_manifest = {
        "schema_version": 1,
        "contract": "macos_dmg_manifest",
        "product": "runtime.zero",
        "version": args.version,
        "target": args.target,
        "source_commit": args.source_commit,
        "source_portable_zip_sha256": archive_sha256,
        "content_sha256": content.hexdigest(),
        "distribution": "github_unsigned_dmg",
        "signature_posture": "unsigned",
        "notarized": False,
        "container_reproducible": False,
        "container_reproducibility_note": "Apple hdiutil emits variable container and filesystem metadata; verify this manifest and the published DMG SHA-256.",
    }
    manifest_bytes = (json.dumps(dmg_manifest, indent=2, sort_keys=True) + "\n").encode("utf-8")
    write_new(staging / "dmg-manifest.json", manifest_bytes, 0o644)
    os.utime(staging, (FIXED_TIME, FIXED_TIME), follow_symlinks=False)
    print(json.dumps(dmg_manifest, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, zipfile.BadZipFile, json.JSONDecodeError) as error:
        print(f"prepare_macos_dmg: {error}", file=sys.stderr)
        raise SystemExit(2) from error
