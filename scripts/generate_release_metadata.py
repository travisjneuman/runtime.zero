#!/usr/bin/env python3
"""Generate deterministic target-filtered SPDX and third-party license evidence."""

from __future__ import annotations

import argparse
from collections import deque
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys
import tomllib

MAX_BINARY_BYTES = 64 * 1024 * 1024
MAX_EVIDENCE_BYTES = 2 * 1024 * 1024
MAX_LICENSE_FILE_BYTES = 128 * 1024
LICENSE_PREFIXES = ("license", "copying", "notice")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", required=True, type=Path)
    parser.add_argument("--target", required=True)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--version", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--source-date", required=True)
    parser.add_argument("--toolchain")
    return parser.parse_args()


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_regular(path: Path, maximum: int) -> bytes:
    metadata = path.lstat()
    if path.is_symlink() or not stat.S_ISREG(metadata.st_mode) or metadata.st_size > maximum:
        raise ValueError(f"unsafe or oversized file: {path.name}")
    data = path.read_bytes()
    if len(data) != metadata.st_size:
        raise ValueError(f"file size changed while reading: {path.name}")
    return data


def write_new(path: Path, data: bytes) -> None:
    if not data or len(data) > MAX_EVIDENCE_BYTES:
        raise ValueError(f"release evidence is empty or exceeds {MAX_EVIDENCE_BYTES} bytes")
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


def spdx_id(name: str, version: str) -> str:
    value = re.sub(r"[^A-Za-z0-9.-]", "-", f"{name}-{version}")
    return f"SPDXRef-Package-{value}"


def source_date(value: str) -> str:
    parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    return parsed.astimezone(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def package_source(package: dict) -> str:
    source = package.get("source")
    repository = package.get("repository")
    if source:
        return source
    if repository:
        return repository
    return "NOASSERTION"


def lock_checksums(repo: Path) -> dict[tuple[str, str, str], str]:
    document = tomllib.loads((repo / "Cargo.lock").read_text(encoding="utf-8"))
    result = {}
    for package in document.get("package", []):
        source = package.get("source")
        checksum = package.get("checksum")
        if source and checksum:
            result[(package["name"], package["version"], source)] = checksum
    return result


def resolved_graph(metadata: dict) -> tuple[dict[str, dict], set[str], list[tuple[str, str]]]:
    packages = {package["id"]: package for package in metadata["packages"]}
    root = next(
        package for package in metadata["packages"]
        if package["name"] == "runtime-zero" and package["version"] == metadata["root_version"]
    )
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    reached = {root["id"]}
    edges: list[tuple[str, str]] = []
    queue = deque([root["id"]])
    while queue:
        parent = queue.popleft()
        for dependency in nodes[parent].get("deps", []):
            kinds = dependency.get("dep_kinds", [])
            if kinds and not any(kind.get("kind") != "dev" for kind in kinds):
                continue
            child = dependency["pkg"]
            edges.append((parent, child))
            if child not in reached:
                reached.add(child)
                queue.append(child)
    return packages, reached, edges


def collect_license_texts(packages: list[dict]) -> tuple[dict[str, list[str]], dict[str, str]]:
    package_hashes: dict[str, list[str]] = {}
    texts: dict[str, str] = {}
    for package in packages:
        manifest_dir = Path(package["manifest_path"]).parent
        candidates: list[Path] = []
        license_file = package.get("license_file")
        if license_file:
            candidates.append(Path(license_file))
        for child in sorted(manifest_dir.iterdir(), key=lambda path: path.name.lower()):
            if child.name.lower().startswith(LICENSE_PREFIXES):
                candidates.append(child)
        hashes = []
        seen_paths = set()
        for candidate in candidates:
            try:
                resolved = candidate.resolve(strict=True)
            except OSError:
                continue
            if resolved in seen_paths or resolved.parent != manifest_dir.resolve(strict=True):
                continue
            seen_paths.add(resolved)
            try:
                data = read_regular(resolved, MAX_LICENSE_FILE_BYTES)
                text = data.decode("utf-8").replace("\r\n", "\n").replace("\r", "\n")
            except (OSError, ValueError, UnicodeDecodeError):
                continue
            text = text.rstrip() + "\n"
            content_hash = sha256(text.encode("utf-8"))
            hashes.append(content_hash)
            texts.setdefault(content_hash, text)
        package_hashes[package["id"]] = sorted(set(hashes))
    return package_hashes, texts


def main() -> int:
    args = parse_args()
    repo = args.repo.resolve(strict=True)
    binary = args.binary.resolve(strict=True)
    if repo not in binary.parents:
        raise ValueError("binary must be inside the repository build tree")
    binary_bytes = read_regular(binary, MAX_BINARY_BYTES)
    if not re.fullmatch(r"[0-9a-f]{40}", args.source_commit):
        raise ValueError("source commit must be a full lowercase Git SHA-1")
    created = source_date(args.source_date)

    if args.toolchain and not re.fullmatch(r"[A-Za-z0-9._-]{1,80}", args.toolchain):
        raise ValueError("toolchain name is invalid")
    command = ["cargo"]
    if args.toolchain:
        command.append(f"+{args.toolchain}")
    command.extend([
        "metadata", "--manifest-path", str(repo / "Cargo.toml"),
        "--locked", "--format-version", "1", "--filter-platform", args.target,
    ])
    completed = subprocess.run(command, cwd=repo, check=True, capture_output=True, text=True)
    metadata = json.loads(completed.stdout)
    metadata["root_version"] = args.version
    packages_by_id, reached, edges = resolved_graph(metadata)
    packages = sorted((packages_by_id[identifier] for identifier in reached), key=lambda p: (p["name"], p["version"], p["id"]))
    identifiers = {
        package["id"]: spdx_id(package["name"], package["version"])
        for package in packages
    }
    if len(set(identifiers.values())) != len(identifiers):
        raise ValueError("resolved packages produce colliding SPDX identifiers")
    checksums = lock_checksums(repo)

    spdx_packages = []
    for package in packages:
        source = package.get("source")
        entry = {
            "SPDXID": identifiers[package["id"]],
            "name": package["name"],
            "versionInfo": package["version"],
            "downloadLocation": package_source(package),
            "filesAnalyzed": False,
            "licenseConcluded": "NOASSERTION",
            "licenseDeclared": package.get("license") or "NOASSERTION",
            "copyrightText": "NOASSERTION",
            "externalRefs": [{
                "referenceCategory": "PACKAGE-MANAGER",
                "referenceType": "purl",
                "referenceLocator": f"pkg:cargo/{package['name']}@{package['version']}",
            }],
        }
        if source:
            checksum = checksums.get((package["name"], package["version"], source))
            if checksum:
                entry["checksums"] = [{"algorithm": "SHA256", "checksumValue": checksum}]
        spdx_packages.append(entry)

    relationships = [{
        "spdxElementId": "SPDXRef-DOCUMENT",
        "relationshipType": "DESCRIBES",
        "relatedSpdxElement": identifiers[next(package["id"] for package in packages if package["name"] == "runtime-zero")],
    }]
    relationships.extend({
        "spdxElementId": identifiers[parent],
        "relationshipType": "DEPENDS_ON",
        "relatedSpdxElement": identifiers[child],
    } for parent, child in sorted(set(edges)) if parent in reached and child in reached)
    root_id = next(identifiers[package["id"]] for package in packages if package["name"] == "runtime-zero")
    relationships.append({
        "spdxElementId": root_id,
        "relationshipType": "CONTAINS",
        "relatedSpdxElement": "SPDXRef-File-rz0",
    })
    relationships.sort(key=lambda item: (item["spdxElementId"], item["relationshipType"], item["relatedSpdxElement"]))

    namespace = f"https://rz0.neuman.dev/spdx/runtime-zero/{args.version}/{args.target}/{args.source_commit}"
    sbom = {
        "spdxVersion": "SPDX-2.3",
        "dataLicense": "CC0-1.0",
        "SPDXID": "SPDXRef-DOCUMENT",
        "name": f"runtime-zero-{args.version}-{args.target}",
        "documentNamespace": namespace,
        "creationInfo": {
            "created": created,
            "creators": ["Organization: runtime.zero contributors", "Tool: runtime.zero-generate-release-metadata-1"],
        },
        "documentDescribes": [root_id],
        "packages": spdx_packages,
        "files": [{
            "SPDXID": "SPDXRef-File-rz0",
            "fileName": "./rz0.exe" if args.target.endswith("windows-msvc") else "./rz0",
            "checksums": [{"algorithm": "SHA256", "checksumValue": sha256(binary_bytes)}],
            "licenseConcluded": "Apache-2.0",
            "copyrightText": "NOASSERTION",
            "comment": f"Final target binary for {args.target}; source commit {args.source_commit}.",
        }],
        "relationships": relationships,
    }
    sbom_bytes = (json.dumps(sbom, indent=2, sort_keys=True) + "\n").encode("utf-8")

    external_packages = [package for package in packages if package.get("source")]
    package_hashes, license_texts = collect_license_texts(external_packages)
    notice = [
        "runtime.zero third-party license evidence\n",
        f"Version: {args.version}\nTarget: {args.target}\nSource commit: {args.source_commit}\n",
        "This generated file inventories declared licenses and available package license/notice texts. It is not legal advice.\n",
        "Packages\n========\n",
    ]
    for package in external_packages:
        hashes = package_hashes[package["id"]]
        notice.append(f"{package['name']} {package['version']}\n  declared: {package.get('license') or 'NOASSERTION'}\n  source: {package_source(package)}\n  text_sha256: {', '.join(hashes) if hashes else 'not-found'}\n")
    notice.append("\nDeduplicated license and notice texts\n=====================================\n")
    for content_hash, text in sorted(license_texts.items()):
        notice.append(f"\n--- sha256:{content_hash} ---\n{text}")
    notice_bytes = "".join(notice).encode("utf-8")

    args.output.mkdir(mode=0o700)
    if args.output.is_symlink() or not args.output.is_dir() or any(args.output.iterdir()):
        raise ValueError("output must be a new empty direct directory")
    write_new(args.output / "SBOM.spdx.json", sbom_bytes)
    write_new(args.output / "THIRD-PARTY-NOTICES.txt", notice_bytes)
    print(json.dumps({
        "sbom_sha256": sha256(sbom_bytes),
        "notices_sha256": sha256(notice_bytes),
        "package_count": len(packages),
        "external_package_count": len(external_packages),
    }, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, KeyError, json.JSONDecodeError, subprocess.CalledProcessError, tomllib.TOMLDecodeError) as error:
        print(f"generate_release_metadata: {error}", file=sys.stderr)
        raise SystemExit(2) from error
