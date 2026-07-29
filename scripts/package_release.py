#!/usr/bin/env python3
"""Create a deterministic, unsigned runtime.zero portable ZIP from a built binary."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import sys
import zipfile

SCHEMA_VERSION = 1
MAX_BINARY_BYTES = 64 * 1024 * 1024
FIXED_ZIP_TIME = (2020, 1, 1, 0, 0, 0)
TARGETS = {
    "aarch64-apple-darwin": "rz0",
    "x86_64-apple-darwin": "rz0",
    "aarch64-pc-windows-msvc": "rz0.exe",
    "x86_64-pc-windows-msvc": "rz0.exe",
    "aarch64-unknown-linux-gnu": "rz0",
    "x86_64-unknown-linux-gnu": "rz0",
}
PUBLIC_FILES = ("README.md", "LICENSE", "SAFETY.md", "SECURITY.md")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", required=True, type=Path)
    parser.add_argument("--target", required=True, choices=sorted(TARGETS))
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--version", required=True)
    parser.add_argument("--source-commit", required=True)
    return parser.parse_args()


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def valid_version(value: str) -> bool:
    return (
        0 < len(value) <= 40
        and value[0].isdigit()
        and all(character.isascii() and (character.isalnum() or character in ".-+") for character in value)
    )


def valid_commit(value: str) -> bool:
    return len(value) == 40 and all(character in "0123456789abcdef" for character in value)


def read_direct_file(path: Path, root: Path, limit: int) -> bytes:
    metadata = path.lstat()
    if path.is_symlink() or not stat.S_ISREG(metadata.st_mode):
        raise ValueError(f"unsafe non-regular input: {path.name}")
    resolved = path.resolve(strict=True)
    if root not in resolved.parents:
        raise ValueError(f"input escaped repository: {path.name}")
    if metadata.st_size > limit:
        raise ValueError(f"input exceeds byte ceiling: {path.name}")
    data = path.read_bytes()
    if len(data) != metadata.st_size:
        raise ValueError(f"input size changed while reading: {path.name}")
    return data


def zip_info(name: str, executable: bool) -> zipfile.ZipInfo:
    info = zipfile.ZipInfo(name, FIXED_ZIP_TIME)
    info.compress_type = zipfile.ZIP_DEFLATED
    info.create_system = 3
    mode = 0o755 if executable else 0o644
    info.external_attr = (stat.S_IFREG | mode) << 16
    return info


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
    repo = args.repo.resolve(strict=True)
    if not valid_version(args.version):
        raise ValueError("version is invalid")
    if not valid_commit(args.source_commit):
        raise ValueError("source commit must be a full lowercase Git SHA-1")
    expected_name = TARGETS[args.target]
    if args.binary.name != expected_name:
        raise ValueError(f"binary name must be {expected_name}")

    binary = read_direct_file(args.binary, repo, MAX_BINARY_BYTES)
    if not binary:
        raise ValueError("binary is empty")
    files: dict[str, tuple[bytes, bool]] = {expected_name: (binary, True)}
    for name in PUBLIC_FILES:
        files[name] = (read_direct_file(repo / name, repo, 2 * 1024 * 1024), False)

    manifest = {
        "schema_version": SCHEMA_VERSION,
        "contract": "release_artifact_manifest",
        "product": "runtime.zero",
        "command": "rz0",
        "version": args.version,
        "target": args.target,
        "source_commit": args.source_commit,
        "distribution": "github_portable_zip",
        "signature_posture": "unsigned",
        "notarized": False,
        "authenticode_signed": False,
        "binary": {
            "path": expected_name,
            "sha256": sha256(binary),
            "size_bytes": len(binary),
        },
        "warning": "Verify SHA-256 before use; this artifact has no paid platform publisher signature.",
    }
    manifest_bytes = (json.dumps(manifest, indent=2, sort_keys=True) + "\n").encode("utf-8")
    files["artifact-manifest.json"] = (manifest_bytes, False)

    args.output.mkdir(parents=True, exist_ok=True)
    if args.output.is_symlink() or not args.output.is_dir():
        raise ValueError("output must be a direct directory")
    archive_name = f"runtime-zero-{args.version}-{args.target}.zip"
    archive = args.output / archive_name
    if archive.exists() or archive.is_symlink():
        raise FileExistsError(f"refusing occupied archive: {archive_name}")

    # Build in memory so the destination appears only after a complete ZIP exists.
    import io

    buffer = io.BytesIO()
    prefix = f"runtime-zero-{args.version}-{args.target}/"
    with zipfile.ZipFile(buffer, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as package:
        for name in sorted(files):
            data, executable = files[name]
            package.writestr(zip_info(prefix + name, executable), data)
    archive_bytes = buffer.getvalue()
    write_new(archive, archive_bytes)

    checksum = f"{sha256(archive_bytes)}  {archive_name}\n".encode("ascii")
    write_new(args.output / f"{archive_name}.sha256", checksum)
    print(json.dumps({
        "archive": str(archive),
        "archive_sha256": sha256(archive_bytes),
        "binary_sha256": manifest["binary"]["sha256"],
        "size_bytes": len(archive_bytes),
    }, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as error:
        print(f"package_release: {error}", file=sys.stderr)
        raise SystemExit(2) from error
