# Inventory Report Contract

> The built-in collector's current implementation, software-identity limits,
> coverage gaps, validation evidence, and restart sequence are summarized in
> [`project-status-and-resumption.md`](project-status-and-resumption.md).

`runtime.zero` uses a versioned inventory contract so platform evidence remains
deterministic, privacy-explicit, and separate from future action planning.

The installed foundation now embeds the bounded first-party collector:

```bash
rz0 apps
rz0 apps --format json
rz0 scan --dry-run --format json
```

`rz0 apps` intentionally omits paths. Scan paths are report-locally redacted by
default; `--include-raw-paths` is an explicit local-only override. The separate
development binary remains available for fixtures and adapter work:

```bash
cargo run -p rz0-module-inventory -- --fixture modules/inventory/tests/fixtures/valid.json --format json
cargo run -p rz0-module-inventory -- --format json --redact-paths
```

The core depends on the source package's library, while the collector remains
independent of the TUI/CLI crate. Both share the owned, strict,
serializable/deserializable model and validator in `crates/inventory-contract/`.
See [`../modules/inventory/README.md`](../modules/inventory/README.md).

## Top-level schema

Schema version `1` includes:

- `schema_version`: currently `1`;
- `contract`: `"inventory_report"`;
- `read_only`: always `true`;
- `writes_attempted`: always `false`;
- `generated_at`: RFC 3339 UTC for live collection and `null` for deterministic
  fixture reports;
- `path_values_redacted`: whether path values use report-local placeholders;
- `raw_registry_keys_included`: always `false` in the current module;
- `host`: OS/architecture plus explicit hostname/current-user privacy flags;
- `runtime`: runtime.zero identity, mode, mutation posture, module-schema
  version, and optional first-party module ID;
- `sources`: independent evidence-source reports;
- `path_entries`: normalized process/user/machine/fixture PATH evidence;
- `tools`: normalized known executable evidence;
- `apps`: normalized opt-in Windows/macOS/Linux application evidence;
- `events`: generic structured source lifecycle events that do not include raw
  evidence values;
- `warnings`: top-level warnings;
- `summary`: deterministic source, PATH, tool, app, event, and total warning
  counts.

JSON field order follows the Rust structure for readable fixtures, but consumers
must use field names rather than object order. Unknown fields fail closed.
Changing the exact field shape requires a new schema version.

## Evidence records

A source record identifies its `id`, `kind`, independent `status`, optional
`duration_ms`, `read_only` posture, and warnings. Statuses are `ok`, `partial`,
`unavailable`, `skipped`, or `error`. One unavailable source does not invalidate
evidence from another source.

PATH records contain `path`, `scope`, `order`, `exists`, `entry_kind`, and
warnings. Windows duplicate comparison is case-insensitive and slash-normalized.
Inputs are bounded to 512 entries per source. Empty/control-character entries,
unsupported kinds, malformed fixture JSON, unknown fixture fields, symlinked
fixture files, and fixtures over 64 KiB fail closed.

Tool records contain normalized identity/category, an exact discovered path,
optional version, source IDs, confidence, and warnings. Discovery checks only a
small allowlist of names directly under PATH entries; it does not recursively
walk drives.

Application records contain normalized name, optional version/publisher/install
location, and a non-secret deterministic evidence ID. Raw uninstall-registry key
names are never emitted. The path-free catalog additionally assigns deterministic
identity groups while preserving every source record and version disagreement;
name-normalized groups can be heuristic and are not permanent product IDs or
action authority.

## Shared validation

The foundation rejects empty or larger-than-16-MiB documents before JSON parsing
and rejects unknown fields, invalid identity/posture metadata, non-read-only
sources, duplicate or absent cross-references, malformed path ordering/kinds,
summary drift, malformed redaction tokens, and collection/warning ceilings.
Current ceilings are 64 sources, 512 PATH entries, 1,024 tools, 4,096 apps, and
8,192 events/warnings. Both core and module JSON render paths run the shared
validator.

`validate_inventory_report` reports base validity separately from
`private_for_export`. A report containing any path-bearing field is private for
export only when every such field uses an exact report-local redaction token.
Raw-path local reports can remain structurally valid but cannot silently satisfy
the export privacy gate. Host/user identity and raw registry keys invalidate
schema 1 entirely.

## Current collectors

| Collector | Default | Boundary |
| --- | --- | --- |
| Process PATH | On | Environment read only; bounded and normalized |
| Windows User/Machine PATH | On for Windows module | `KEY_READ` registry access only |
| Known executable discovery | On | Exact allowlisted filenames under PATH only |
| Known executable version probes | Off | Explicit `--probe-versions`; exact path, symlink/reparse-component rejection, static arguments, no shell, cleared environment, `/` working directory, shared descriptor audit/Unix process-group teardown, atomic 2-second deadline, 64 KiB per-stream capture; Windows fails closed pending race-free containment |
| Windows installed applications | Off | Explicit `--include-apps`; standard uninstall views, read only, 4,096-record cap |
| macOS application bundles | On in installed core; opt-in in development binary | Direct `.app` directories under five known roots; bounded `Info.plist` reads provide versions |
| macOS Homebrew metadata | On in installed core; opt-in in development binary | Direct Cellar/Caskroom directories only; no manager process or network access |
| Linux desktop entries | On in installed core; opt-in in development binary | Regular XDG `.desktop` files up to 64 KiB, user/hidden precedence, no `Exec` output or execution |
| Other package-manager listings/catalogs | Off | Deferred because behavior can vary by version, locale, source agreements, and network access |

Script-based executable probes remain disabled. The module detects package manager executables but does not invoke manager
list/update/install/uninstall commands. Application collectors reject symlinked
roots and records and cap entry inspection and normalized output at 4,096 records. Linux desktop
files use XDG data-root precedence; a higher-priority `Hidden=true` entry blocks
the same lower-priority desktop ID. The parser does not treat the desktop-spec
`Version` key as an application version.

## Privacy and sharing

- Hostname and current user are omitted.
- Raw registry keys are omitted.
- Local paths can contain usernames or private project names. Use
  `--redact-paths` before sharing output.
- Redaction replaces PATH entries, executable paths, and app install locations
  with stable report-local placeholders. It does not redact application names,
  versions, or publishers; opt-in app reports require separate review before
  sharing.
- Structured events contain source IDs/status only, never raw path/app values.
- Credentials, OAuth sessions, browser profiles, project contents, backups, and
  unknown user data are outside the inventory contract.
- Public examples and fixtures use synthetic paths only.

## Non-goals

Inventory evidence is not an instruction or trust decision. This layer does not
write PATH/registry state, install/update/uninstall software, clean files, run
package managers, fetch remote metadata, or approve
third-party modules.

## Implementation references

- Rust `std::env::split_paths`: https://doc.rust-lang.org/std/env/fn.split_paths.html
- Rust `std::process::Command`: https://doc.rust-lang.org/std/process/struct.Command.html
- Microsoft registry access rights (`KEY_READ` / `KEY_QUERY_VALUE`): https://learn.microsoft.com/windows/win32/sysinfo/registry-key-security-and-access-rights
- Microsoft registry value types: https://learn.microsoft.com/windows/win32/sysinfo/registry-value-types
- `winreg` crate documentation: https://docs.rs/winreg
- Apple bundle structures: https://developer.apple.com/library/archive/documentation/CoreFoundation/Conceptual/CFBundles/BundleTypes/BundleTypes.html
- Desktop Entry Specification keys: https://specifications.freedesktop.org/desktop-entry-spec/latest/recognized-keys.html
- XDG Base Directory Specification: https://specifications.freedesktop.org/basedir-spec/latest/

These references describe APIs; they do not replace fixture/runtime verification.

## Remaining proof gates

The code is fixture-tested on macOS and cross-checked for the Windows MSVC
target. Before claiming Windows support, it still needs race-free handle audit/process
containment plus a real Windows runtime smoke covering persisted PATH, registry
views, app normalization, timeout behavior, redaction, and the installed
terminal experience. The macOS app
adapter was exercised on the development host and Linux parser behavior is
fixture-tested plus Linux-target compiled, but a real Linux runtime and broader
non-Homebrew macOS/Linux package-manager, service, and persistence inventory
remain later adapter work.
