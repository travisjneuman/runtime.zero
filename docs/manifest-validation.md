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
- `safety`.

The `safety` object declares:

- `mutates_system`;
- `requires_confirmation`;
- `dry_run_required`;
- `quarantine_supported`;
- `remote_execution_allowed`.

Unknown fields are rejected so module authors cannot rely on undeclared
behavior.

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

Directory loading is intentionally shallow: `rz0 modules --from <directory>`
loads JSON files directly in that directory only. Valid manifests are listed as
installed modules; invalid manifests remain validation reports. Duplicate
installed module IDs are treated as validation errors so the registry never has
to choose between competing manifests silently.

## Safety non-goals

This validation layer does not yet provide signing, checksums, sandboxing,
revocation, module installation, remote distribution, update orchestration, or
third-party trust. Those require separate approval and threat modeling.
