#!/usr/bin/env python3
"""Verify a runtime.zero portable ZIP and its embedded artifact manifest."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import stat
import sys
import zipfile

MAX_ARCHIVE_BYTES = 128 * 1024 * 1024
MAX_ENTRY_BYTES = 64 * 1024 * 1024
MAX_ENTRIES = 16
REQUIRED_PUBLIC_FILES = {
    "LICENSE",
    "README.md",
    "SAFETY.md",
    "SECURITY.md",
    "SBOM.spdx.json",
    "THIRD-PARTY-NOTICES.txt",
    "artifact-manifest.json",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", required=True, type=Path)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--target")
    parser.add_argument("--expected-archive-sha256")
    parser.add_argument("--checksum-file", type=Path)
    return parser.parse_args()


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def valid_digest(value: object) -> bool:
    return isinstance(value, str) and len(value) == 64 and all(
        character in "0123456789abcdef" for character in value
    )


def validate_commit(value: str) -> None:
    if len(value) != 40 or any(character not in "0123456789abcdef" for character in value):
        raise ValueError("source commit must be a full lowercase Git SHA-1")


def safe_member_name(name: str) -> tuple[str, str]:
    if not name or "\\" in name:
        raise ValueError(f"unsafe ZIP member name: {name!r}")
    path = PurePosixPath(name)
    if path.is_absolute() or any(part in ("", ".", "..") for part in path.parts):
        raise ValueError(f"unsafe ZIP member path: {name!r}")
    if len(path.parts) != 2:
        raise ValueError(f"ZIP member must be directly inside its release root: {name!r}")
    return path.parts[0], path.parts[1]


def read_checksum(path: Path, archive_name: str) -> str:
    text = path.read_text(encoding="ascii")
    fields = text.strip().split()
    if len(fields) != 2 or fields[1] != archive_name or not valid_digest(fields[0]):
        raise ValueError("checksum file does not contain one valid archive checksum")
    return fields[0]


def verify(args: argparse.Namespace) -> dict[str, object]:
    validate_commit(args.source_commit)
    archive = args.archive.resolve(strict=True)
    metadata = archive.stat()
    if not stat.S_ISREG(metadata.st_mode) or archive.is_symlink():
        raise ValueError("archive must be a regular non-symlink file")
    if metadata.st_size == 0 or metadata.st_size > MAX_ARCHIVE_BYTES:
        raise ValueError("archive is empty or exceeds the release evidence ceiling")
    archive_bytes = archive.read_bytes()
    archive_digest = sha256(archive_bytes)
    if args.expected_archive_sha256 and archive_digest != args.expected_archive_sha256:
        raise ValueError("archive SHA-256 does not match the expected digest")
    if args.checksum_file:
        if read_checksum(args.checksum_file.resolve(strict=True), archive.name) != archive_digest:
            raise ValueError("archive SHA-256 does not match the checksum file")

    with zipfile.ZipFile(archive) as package:
        infos = package.infolist()
        if not infos or len(infos) > MAX_ENTRIES:
            raise ValueError("ZIP entry count is empty or exceeds the release ceiling")
        members: dict[str, bytes] = {}
        roots: set[str] = set()
        for info in infos:
            root, basename = safe_member_name(info.filename)
            roots.add(root)
            if info.is_dir() or stat.S_ISLNK((info.external_attr >> 16) & 0xFFFF):
                raise ValueError(f"ZIP member is not a regular file: {info.filename!r}")
            if info.file_size > MAX_ENTRY_BYTES:
                raise ValueError(f"ZIP member exceeds the release ceiling: {info.filename!r}")
            if basename in members:
                raise ValueError(f"duplicate ZIP member basename: {basename!r}")
            data = package.read(info)
            if len(data) != info.file_size:
                raise ValueError(f"ZIP member size changed while reading: {info.filename!r}")
            members[basename] = data
        if len(roots) != 1:
            raise ValueError("ZIP must contain exactly one release root")

        missing = REQUIRED_PUBLIC_FILES - members.keys()
        if missing:
            raise ValueError(f"ZIP is missing required members: {', '.join(sorted(missing))}")
        try:
            manifest = json.loads(members["artifact-manifest.json"])
        except json.JSONDecodeError as error:
            raise ValueError(f"artifact manifest is not valid JSON: {error}") from error
        if not isinstance(manifest, dict):
            raise ValueError("artifact manifest must be a JSON object")

        if manifest.get("contract") != "release_artifact_manifest":
            raise ValueError("artifact manifest contract is invalid")
        if manifest.get("source_commit") != args.source_commit:
            raise ValueError("artifact manifest source commit does not match the requested commit")
        target = manifest.get("target")
        if not isinstance(target, str) or not target:
            raise ValueError("artifact manifest target is missing")
        if args.target and target != args.target:
            raise ValueError("artifact manifest target does not match the requested target")
        binary = manifest.get("binary")
        sbom = manifest.get("sbom")
        notices = manifest.get("third_party_notices")
        for label, descriptor in (("binary", binary), ("sbom", sbom), ("third_party_notices", notices)):
            if not isinstance(descriptor, dict):
                raise ValueError(f"artifact manifest {label} descriptor is invalid")
            path = descriptor.get("path")
            if not isinstance(path, str) or path not in members:
                raise ValueError(f"artifact manifest {label} path is not a ZIP member")
            data = members[path]
            if descriptor.get("size_bytes") != len(data) or descriptor.get("sha256") != sha256(data):
                raise ValueError(f"artifact manifest {label} digest or size does not match ZIP bytes")
        if binary.get("path") != target_binary_name(target):
            raise ValueError("artifact manifest binary path does not match target")
        if manifest.get("signature_posture") != "unsigned" or manifest.get("notarized") is not False:
            raise ValueError("artifact manifest signature posture is not the declared unsigned posture")

    return {
        "archive": str(archive),
        "archive_sha256": archive_digest,
        "source_commit": args.source_commit,
        "target": target,
        "entry_count": len(members),
        "decision": "pass",
    }


def target_binary_name(target: str) -> str:
    return "rz0.exe" if "windows" in target else "rz0"


def main() -> int:
    args = parse_args()
    result = verify(args)
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, zipfile.BadZipFile) as error:
        print(f"verify_release_package: {error}", file=sys.stderr)
        raise SystemExit(2) from error
