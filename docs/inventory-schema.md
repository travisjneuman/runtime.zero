# Inventory Report Contract

`runtime.zero` is beginning the Phase 2 inventory lane with a versioned output
contract before adding platform probes. The first surface is:

```bash
rz0 scan --dry-run --format json
```

This command currently emits an empty schema-1 `inventory_report`. It does not
read PATH, registry, package-manager, application, or executable evidence yet.
That deliberately small contract-first slice gives fixture tests and the future
first-party Windows inventory module a deterministic target without bundling a
feature module into the core or enabling module execution.

## Top-level schema

Schema version `1` has these fields:

- `schema_version`: currently `1`;
- `contract`: `"inventory_report"`;
- `read_only`: always `true` in this lane;
- `writes_attempted`: always `false`;
- `generated_at`: `null` until a real collector runs; future reports may use an
  ISO-8601 timestamp supplied through a testable clock boundary;
- `host`: OS/architecture plus explicit hostname/current-user privacy flags;
- `runtime`: runtime.zero identity, dry-run mode, disabled mutation capability,
  and module-schema version;
- `sources`: independent evidence-source reports;
- `path_entries`: normalized process/user/machine PATH evidence;
- `tools`: normalized executable/tool evidence;
- `apps`: normalized application/package evidence;
- `warnings`: top-level unavailable/partial/privacy warnings;
- `summary`: deterministic source, PATH, tool, app, and warning counts.

JSON field order follows the Rust structure for readable fixtures, but consumers
must use field names rather than object order. Contract changes should be
additive within schema version `1`; incompatible changes require a new schema
version.

## Planned evidence records

A source record will identify its `id`, `kind`, independent `status`, optional
`duration_ms`, `read_only` posture, and warnings. Source statuses should be one
of `ok`, `partial`, `unavailable`, `skipped`, or `error` once collectors exist.
One unavailable source must not invalidate evidence from another source.

PATH records reserve `path`, `scope`, `order`, `exists`, `entry_kind`, and
warnings. Tool records reserve normalized identity/category, optional local
executable path/version, source IDs, confidence, and warnings. Application
records reserve normalized identity, source, optional version/publisher/install
location, and warnings.

These are evidence records, not instructions. They do not authorize command
execution, updates, installs, removals, PATH edits, registry writes, cleanup, or
module activation.

## Privacy and safety

- Hostname and current user are omitted by default.
- No credentials, sessions, browser profiles, project workspaces, backups, or
  unknown user data may be inspected.
- Raw Windows registry keys are not part of the default report.
- Local executable/install paths may be useful in local output, but sanitized
  fixtures and a redaction/export policy are required before share-oriented
  output is added.
- Collectors must be read-only, independently optional, timeout-bounded where
  commands are eventually involved, and fixture-tested.
- No network access, package-manager list command, persisted PATH registry read,
  or live executable version probe is part of this contract-only slice.

## Next implementation gate

The next safe slice is fixture-backed Windows process-PATH parsing and
normalization. Valid, duplicate, missing, malformed, and unsupported-platform
fixtures should prove deterministic output and fail-closed behavior before live
Windows probes are added. Persisted PATH, executable-version probes,
package-manager listings, and app registry evidence remain later independent
gates.
