import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import zipfile


SCRIPT = Path(__file__).parents[1] / "verify_release_package.py"
SPEC = importlib.util.spec_from_file_location("verify_release_package", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class VerifyReleasePackageTests(unittest.TestCase):
    def make_archive(self, root: Path, *, tamper_binary: bool = False) -> tuple[Path, Path]:
        target = "aarch64-apple-darwin"
        source_commit = "a" * 40
        binary = b"release binary\n"
        sbom = b'{"spdxVersion":"SPDX-2.3"}\n'
        notices = b"notices\n"
        if tamper_binary:
            binary = b"tampered\n"
        manifest = {
            "schema_version": 1,
            "contract": "release_artifact_manifest",
            "product": "runtime.zero",
            "command": "rz0",
            "version": "0.1.0",
            "target": target,
            "source_commit": source_commit,
            "signature_posture": "unsigned",
            "notarized": False,
            "binary": {
                "path": "rz0",
                "sha256": hashlib.sha256(b"release binary\n").hexdigest(),
                "size_bytes": len(b"release binary\n"),
            },
            "sbom": {
                "path": "SBOM.spdx.json",
                "sha256": hashlib.sha256(sbom).hexdigest(),
                "size_bytes": len(sbom),
            },
            "third_party_notices": {
                "path": "THIRD-PARTY-NOTICES.txt",
                "sha256": hashlib.sha256(notices).hexdigest(),
                "size_bytes": len(notices),
            },
        }
        members = {
            "LICENSE": b"license\n",
            "README.md": b"readme\n",
            "SAFETY.md": b"safety\n",
            "SECURITY.md": b"security\n",
            "SBOM.spdx.json": sbom,
            "THIRD-PARTY-NOTICES.txt": notices,
            "artifact-manifest.json": (json.dumps(manifest) + "\n").encode(),
            "rz0": binary,
        }
        archive = root / "release.zip"
        with zipfile.ZipFile(archive, "w") as package:
            for name, data in members.items():
                package.writestr(f"runtime-zero-0.1.0-{target}/{name}", data)
        checksum = root / "release.zip.sha256"
        checksum.write_text(f"{hashlib.sha256(archive.read_bytes()).hexdigest()}  {archive.name}\n")
        return archive, checksum

    def test_valid_package_passes(self):
        with tempfile.TemporaryDirectory() as directory:
            archive, checksum = self.make_archive(Path(directory))
            result = MODULE.verify(
                type("Args", (), {
                    "archive": archive,
                    "source_commit": "a" * 40,
                    "target": "aarch64-apple-darwin",
                    "expected_archive_sha256": None,
                    "checksum_file": checksum,
                })()
            )
            self.assertEqual(result["decision"], "pass")
            self.assertEqual(result["entry_count"], 8)

    def test_manifest_digest_drift_fails_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            archive, checksum = self.make_archive(Path(directory), tamper_binary=True)
            with self.assertRaisesRegex(ValueError, "binary digest or size"):
                MODULE.verify(
                    type("Args", (), {
                        "archive": archive,
                        "source_commit": "a" * 40,
                        "target": "aarch64-apple-darwin",
                        "expected_archive_sha256": None,
                        "checksum_file": checksum,
                    })()
                )


if __name__ == "__main__":
    unittest.main()
