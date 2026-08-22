# Module Manifest Validation

`runtime.zero` can validate local module manifests before any module execution
model exists. This is a foundation contract, not a module installer.

## Commands

```bash
rz0 modules validate <manifest.json>
rz0 modules validate <manifest.json> --format json
rz0 modules status [--store-root <path>]
rz0 modules status --store-root <path> --format json
rz0 modules --from <directory>
rz0 modules --from <directory> --format json
rz0 modules install --dry-run <package-dir-or-manifest>
rz0 modules install --dry-run <package-dir-or-manifest> --format json
rz0 modules trust verify --manifest <manifest.json> --signature <envelope.json> --trusted-test-key <key.json>
rz0 modules lifecycle-plan install --dry-run --module-id first-party.inventory \
  --from-state absent --to-state installed_inactive --to-version 0.1.0
```

The loader is read-only. It reads JSON metadata from the local filesystem and
returns validation results and dry-run plans. It does not fetch remote content,
install modules, enable modules, run module code, or repair invalid manifests.

`modules trust verify` is the bounded bridge between package integrity and the
existing `crates/module-trust/` test-key contract. It reads the exact local
manifest bytes once, validates the manifest and explicitly listed package files,
binds the detached envelope to the manifest ID, version, and SHA-256, and
verifies it against a caller-selected non-revoked public test key. The result is
review evidence only: it never installs, stages, activates, invokes, or
authorizes a module.

Lifecycle review is separate from manifest loading. The
`modules lifecycle-plan` command renders the crate-owned transition grammar,
including its ordered gates and digest, but it does not publish or execute any
state transition.

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
- optional schema-1 `permissions`;
- optional `integrity`.

The `safety` object declares:

- `mutates_system`;
- `requires_confirmation`;
- `dry_run_required`;
- `quarantine_supported`;
- `remote_execution_allowed`.

Unknown fields are rejected so module authors cannot rely on undeclared
behavior.

## Read-only permission metadata

The optional schema-1 `permissions` object is the first enforceable declaration
for read-only modules. Permission names come from the shared foundation
capability vocabulary in [`capability-contract.md`](capability-contract.md),
while this schema accepts only its read-only subset. It separates all `declared`
permissions into
`default_grants` and `explicit_grants`. Current known permissions are:

- `process_environment_read`;
- `filesystem_metadata_read`;
- `persisted_environment_registry_read`;
- `application_registry_read`;
- `application_filesystem_read`;
- `exact_command_probe`.

Lists must be duplicate-free, default and explicit grants must be disjoint, and
every grant must be declared. Application registry/filesystem reads and exact
command probes must be explicit, never default. Schema 1 rejects mutating modules and
action/mutation capabilities and unknown future permissions. First-party
manifests without permissions remain
valid for compatibility but receive a warning that they have no enforceable
permission declaration.

This is validation metadata, not an execution grant: the core still does not
load or execute module code.

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

An optional `complete_file_set: true` integrity flag enables the stricter
package-review mode. It recursively enumerates bounded regular files beneath
the manifest directory, rejects symlinks/reparse points, undeclared files,
unsupported file types, more than 128 files, more than 16 directory levels, or
more than 512 MiB of package bytes. The root manifest is excluded from the
payload list because hashing its own integrity field would be circular; the
read-only trust command separately binds its exact manifest bytes to the
detached signature envelope.

Provenance metadata is consistency-checked, not trusted: source, publisher,
release ID, and repository fields are bounded, and provenance publisher must
match the manifest publisher. This does not establish release origin,
reproducibility, freshness, revocation, or production signer authority.

## Current validation rules

- Manifest files must be regular files and at most 64 KiB.
- IDs must use lowercase ASCII letters, digits, dots, and hyphens.
- `core.*` IDs are reserved for foundation manifests.
- Supported platforms are currently `windows`, `macos`, and `linux`.
- First-party modules must be published by `runtime.zero`.
- Third-party modules are rejected until the trust model exists.
- `remote_execution_allowed` must be `false`.
- Permission schema 1 is read-only; application inventory reads and command probes must be explicit.
- Mutating modules must require confirmation and dry-run support.
- Destructive-gated modules must support quarantine or rollback.
- Installed manifests must include integrity metadata.
- Planned manifests without integrity metadata remain valid with a warning.
- Integrity metadata may list at most 128 files.
- Each listed file must be at most 64 MiB.
- `complete_file_set` can require all regular package files (except the root
  manifest) to be explicitly listed.
- Complete package review is bounded at 128 files, 16 directory levels, and
  512 MiB total.
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

`rz0 modules status` reads the installed registry, receipt inventory, and
installed manifest/package integrity evidence through the same bounded
validators. It is the status surface for deciding whether an installed record
is `installed_inactive`, `active`, or `degraded`; it never executes a module or
changes registry/receipt state. Its optional `--store-root` argument is a local
read-only inspection override.

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

## Safety non-goals and the bounded v0 path

This core manifest-validation layer remains read-only and does not perform
remote distribution, update orchestration, sandboxing, or third-party trust.
The separate `modules trust verify` review command remains an evidence path.

The signed v0 CLI path is the deliberately narrower exception: it accepts only
the source-only, read-only `first-party.inventory` package on macOS, verifies a
caller-selected non-revoked `first_party_release` key and exact manifest
identity, and then hands verified bytes to the foundation-owned store/lifecycle
executor. The executor owns confirmation, receipts, active/inactive registry
state, update, quarantine, recovery, and the bounded process host. It does not
fetch packages or establish a bundled production trust root. Key custody,
provenance transparency, rotation/revocation, native sandboxing, and other
module families require separate implementation and acceptance.
