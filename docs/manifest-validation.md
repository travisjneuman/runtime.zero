# Module Manifest Validation

`runtime.zero` can validate local module manifests before any module execution
model exists. This is a foundation contract, not a module installer.

## Commands

```bash
rz0 modules validate <manifest.json>
rz0 modules validate <manifest.json> --format json
rz0 modules --from <directory>
rz0 modules --from <directory> --format json
```

The loader is read-only. It reads JSON metadata from the local filesystem and
returns validation results. It does not fetch remote content, install modules,
enable modules, run module code, or repair invalid manifests.

## Manifest shape

Schema version `1` currently expects:

- `manifest_version`;
- `id`;
- `display_name`;
- `version`;
- `publisher`;
- `kind`;
- `status`;
- `summary`;
- `capabilities`;
- `supported_platforms`;
- `risk_level`;
- `safety`;
- optional `integrity`.

The `safety` object declares:

- `mutates_system`;
- `requires_confirmation`;
- `dry_run_required`;
- `quarantine_supported`;
- `remote_execution_allowed`.

Unknown fields are rejected so module authors cannot rely on undeclared
behavior.

## Package integrity metadata

Installed manifests must include local package integrity metadata. Planned
manifests may omit it, but validation emits a warning because the package has
not been verified. Only SHA-256 directory packages rooted at the manifest
directory are supported in this slice.

```json
{
  "integrity": {
    "package_format": "directory",
    "root_policy": "manifest_directory",
    "hash_algorithm": "sha256",
    "files": [
      {
        "path": "payload.txt",
        "sha256": "1520b869efef13352d18285a6e072ab1e7f7f771ece09f5f84d603c5310c2621",
        "size_bytes": 29,
        "role": "payload"
      }
    ],
    "provenance": {
      "source": "local_fixture",
      "publisher": "runtime.zero",
      "release_id": "fixture"
    }
  }
}
```

Integrity validation is local and read-only. It opens only explicitly listed
files under the manifest directory, hashes them with SHA-256, and compares the
result to manifest metadata. It never fetches remote packages, runs package
code, loads dynamic libraries, runs scripts or hooks, repairs files, installs
modules, updates modules, or removes modules.

## Current validation rules

- Manifest files must be regular files and at most 64 KiB.
- IDs must use lowercase ASCII letters, digits, dots, and hyphens.
- `core.*` IDs are reserved for foundation manifests.
- Supported platforms are currently `windows`, `macos`, and `linux`.
- First-party modules must be published by `runtime.zero`.
- Third-party modules are rejected until the trust model exists.
- `remote_execution_allowed` must be `false`.
- Mutating modules must require confirmation and dry-run support.
- Destructive-gated modules must support quarantine or rollback.
- Installed manifests must include integrity metadata.
- Planned manifests without integrity metadata remain valid with a warning.
- Integrity metadata may list at most 128 files.
- Each listed file must be at most 64 MiB.
- Listed paths must be relative manifest-directory paths.
- Absolute paths, `..` traversal, URL-like paths, backslash paths, duplicate
  paths, malformed SHA-256 values, missing files, size mismatches, hash
  mismatches, symlinks, reparse points, and non-file paths are rejected.
- SHA-256 is the only supported hash algorithm.

Directory loading is intentionally shallow: `rz0 modules --from <directory>`
loads JSON files directly in that directory only. Valid manifests are listed as
installed modules; invalid manifests remain validation reports. Duplicate
installed module IDs are treated as validation errors so the registry never has
to choose between competing manifests silently.

## Safety non-goals

This validation layer does not yet provide signature verification, revocation,
module installation, remote distribution, update orchestration, sandboxing, or
third-party trust. Those require separate approval and threat modeling.
