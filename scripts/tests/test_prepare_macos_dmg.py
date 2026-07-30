from __future__ import annotations

import hashlib
import json
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import unittest
import zipfile

REPO = Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts" / "prepare_macos_dmg.py"
TARGET = "aarch64-apple-darwin"
VERSION = "0.1.0"
COMMIT = "a" * 40
PREFIX = f"runtime-zero-{VERSION}-{TARGET}/"


class PrepareMacosDmgTests(unittest.TestCase):
    def test_valid_archive_prepares_exact_private_content(self) -> None:
        with tempfile.TemporaryDirectory(prefix="rz0-dmg-test-") as temporary:
            root = Path(temporary)
            archive, checksum = make_archive(root)
            result = run_prepare(root, archive, checksum)
            self.assertEqual(result.returncode, 0, result.stderr)
            staging = root / "staging"
            self.assertEqual(
                {path.name for path in staging.iterdir()},
                {
                    "rz0",
                    "README.md",
                    "LICENSE",
                    "SAFETY.md",
                    "SECURITY.md",
                    "artifact-manifest.json",
                    "SBOM.spdx.json",
                    "THIRD-PARTY-NOTICES.txt",
                    "dmg-manifest.json",
                },
            )
            manifest = json.loads((staging / "dmg-manifest.json").read_text())
            self.assertFalse(manifest["container_reproducible"])
            self.assertEqual(manifest["source_commit"], COMMIT)
            self.assertEqual(stat.S_IMODE((staging / "rz0").stat().st_mode), 0o755)

    def test_checksum_mismatch_fails_before_staging(self) -> None:
        with tempfile.TemporaryDirectory(prefix="rz0-dmg-test-") as temporary:
            root = Path(temporary)
            archive, checksum = make_archive(root)
            checksum.write_text(f"{'0' * 64}  {archive.name}\n")
            result = run_prepare(root, archive, checksum)
            self.assertEqual(result.returncode, 2)
            self.assertFalse((root / "staging").exists())

    def test_extra_or_traversal_entry_fails_closed(self) -> None:
        for entry in (PREFIX + "extra.txt", PREFIX + "../escape.txt"):
            with self.subTest(entry=entry), tempfile.TemporaryDirectory(
                prefix="rz0-dmg-test-"
            ) as temporary:
                root = Path(temporary)
                archive, checksum = make_archive(root, extra=(entry, b"bad", 0o100644))
                result = run_prepare(root, archive, checksum)
                self.assertEqual(result.returncode, 2)
                self.assertFalse((root / "staging").exists())

    def test_symlink_entry_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory(prefix="rz0-dmg-test-") as temporary:
            root = Path(temporary)
            archive, checksum = make_archive(
                root,
                override=(PREFIX + "README.md", b"rz0", stat.S_IFLNK | 0o777),
            )
            result = run_prepare(root, archive, checksum)
            self.assertEqual(result.returncode, 2)
            self.assertFalse((root / "staging").exists())


def make_archive(
    root: Path,
    *,
    extra: tuple[str, bytes, int] | None = None,
    override: tuple[str, bytes, int] | None = None,
) -> tuple[Path, Path]:
    binary = b"synthetic-rz0-binary"
    sbom = b'{"spdxVersion":"SPDX-2.3"}\n'
    notices = b"synthetic third-party notices\n"
    manifest = {
        "schema_version": 1,
        "contract": "release_artifact_manifest",
        "version": VERSION,
        "target": TARGET,
        "source_commit": COMMIT,
        "distribution": "github_portable_zip",
        "signature_posture": "unsigned",
        "notarized": False,
        "binary": {
            "sha256": hashlib.sha256(binary).hexdigest(),
            "size_bytes": len(binary),
        },
        "sbom": {
            "sha256": hashlib.sha256(sbom).hexdigest(),
            "size_bytes": len(sbom),
        },
        "third_party_notices": {
            "sha256": hashlib.sha256(notices).hexdigest(),
            "size_bytes": len(notices),
        },
    }
    files = {
        "rz0": (binary, stat.S_IFREG | 0o755),
        "README.md": (b"readme", stat.S_IFREG | 0o644),
        "LICENSE": (b"license", stat.S_IFREG | 0o644),
        "SAFETY.md": (b"safety", stat.S_IFREG | 0o644),
        "SECURITY.md": (b"security", stat.S_IFREG | 0o644),
        "SBOM.spdx.json": (sbom, stat.S_IFREG | 0o644),
        "THIRD-PARTY-NOTICES.txt": (notices, stat.S_IFREG | 0o644),
        "artifact-manifest.json": (
            (json.dumps(manifest) + "\n").encode(),
            stat.S_IFREG | 0o644,
        ),
    }
    if override is not None:
        path, data, mode = override
        files[path[len(PREFIX):]] = (data, mode)
    archive = root / f"runtime-zero-{VERSION}-{TARGET}.zip"
    with zipfile.ZipFile(archive, "w") as package:
        for name, (data, mode) in files.items():
            info = zipfile.ZipInfo(PREFIX + name, (2020, 1, 1, 0, 0, 0))
            info.create_system = 3
            info.external_attr = mode << 16
            package.writestr(info, data)
        if extra is not None:
            name, data, mode = extra
            info = zipfile.ZipInfo(name, (2020, 1, 1, 0, 0, 0))
            info.create_system = 3
            info.external_attr = mode << 16
            package.writestr(info, data)
    archive_sha256 = hashlib.sha256(archive.read_bytes()).hexdigest()
    checksum = archive.with_suffix(".zip.sha256")
    checksum.write_text(f"{archive_sha256}  {archive.name}\n")
    return archive, checksum


def run_prepare(root: Path, archive: Path, checksum: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable,
            str(SCRIPT),
            "--archive",
            str(archive),
            "--checksum",
            str(checksum),
            "--staging",
            str(root / "staging"),
            "--target",
            TARGET,
            "--version",
            VERSION,
            "--source-commit",
            COMMIT,
        ],
        text=True,
        capture_output=True,
        check=False,
    )


if __name__ == "__main__":
    unittest.main()
