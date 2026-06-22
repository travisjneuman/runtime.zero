# Module Manifest Validation

`runtime.zero` can validate local module manifests before any module execution
model exists. This is a foundation contract, not a module installer.

## Commands

```bash
rz0 modules validate <manifest.json>
rz0 modules validate <manifest.json> --format json
rz0 modules --from <directory>
rz0 modules --from <directory> --format json
rz0 modules install --dry-run <package-dir-or-manifest>
rz0 modules install --dry-run <package-dir-or-manifest> --format json
```

The loader is read-only. It reads JSON metadata from the local filesystem and
returns validation results and dry-run plans. It does not fetch remote content,
install modules, enable modules, run module code, or repair invalid manifests.

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

## Dry-run install planner

`rz0 modules install --dry-run <package-dir-or-manifest>` is the first
planner-only install surface. It accepts either:

- a local package directory containing `rz0-module.json`; or
- a direct local manifest path.

The planner first runs the same manifest and package integrity checks described
above. Valid package plans report:

- the manifest path;
- the local package root;
- the proposed module install root;
- the proposed module directory;
- planned actions for directory creation, verified file copy, and manifest
  recording.

These actions are descriptions only. Text output says `writes_attempted: no`,
and JSON output sets every action to `would_write: false`. Invalid manifests or
integrity failures return a nonzero exit code and no planned actions.

JSON dry-run output also includes read-only future-state contract metadata:

- `store.store_schema_version`;
- `store.data_root`;
- `store.state_root`;
- `store.registry_path`;
- `store.receipt_path`;
- `store.transaction_path`;
- `store.rollback_supported`;
- `store.quarantine_supported`;
- `store.forbidden_path_classes`;
- `launch_context.launch_mode`.

These fields are calculated only. They do not create store directories or write
registry, receipt, transaction, staging, rollback, quarantine, or module files.

## Safety non-goals

This validation layer does not yet provide signature verification, revocation,
module installation, remote distribution, update orchestration, sandboxing, or
third-party trust. Those require separate approval and threat modeling.
