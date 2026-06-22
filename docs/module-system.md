# Module System

Modules are the unit of growth for `runtime.zero`. The foundation should remain useful with zero optional modules installed.

A module manifest declares:

- manifest version;
- id and display name;
- version and publisher;
- module kind;
- lifecycle status;
- supported platforms;
- capabilities;
- risk level;
- whether it mutates the system;
- confirmation requirements;
- dry-run requirements;
- quarantine/rollback support;
- remote execution policy;
- optional local package integrity metadata;
- test fixtures.

## Design rule

Every module must be safe to run in discovery/dry-run mode before it is allowed to mutate anything.

Core primitives are not feature modules. `core.cli`, `core.policy`, and
`core.registry` describe the foundation. Optional modules are listed separately
and are not bundled, installed, or executed by default.

## Current registry surface

```bash
rz0 modules
rz0 modules --format json
rz0 modules validate <manifest.json>
rz0 modules --from <directory> --format json
rz0 modules install --dry-run <package-dir-or-manifest>
rz0 store plan
rz0 store plan --format json
rz0 store status
rz0 store status --format json
```

The JSON output uses schema version `1` and separates:

- `core`;
- `installed_modules`;
- `planned_module_families`.

An empty `installed_modules` list is valid and expected for the foundation-only build.

`rz0 modules validate` reads one local JSON manifest and reports whether it
passes the foundation contract. `rz0 modules --from <directory>` reads JSON
manifests directly inside that directory and includes only valid manifests in
`installed_modules`. Neither command executes code or fetches remote content.

Installed manifests are valid only when their explicitly listed package files
pass local SHA-256 integrity checks. Planned manifests may omit integrity
metadata, but the validator reports that they are not package-verified yet.
The first integrity slice supports only local directory packages rooted at the
manifest directory; it rejects absolute paths, traversal, URLs, symlinks,
reparse points, files over 64 MiB, and manifests with more than 128 listed
files.

`rz0 modules install --dry-run <package-dir-or-manifest>` is a planner only.
It accepts a local package directory containing `rz0-module.json`, or a direct
local manifest path, then reuses manifest and package integrity validation. If
the package is valid, it reports proposed install state such as the module
directory, verified files that would be copied later, and the manifest metadata
that would be recorded later. Every planned action has `would_write: false` in
JSON output. The command performs no writes and intentionally has no non-dry-run
form.

Dry-run JSON now also includes a `store` object and `launch_context` object.
The `store` object describes future user-local data/state/cache/log/quarantine
paths, registry/receipt/transaction paths, rollback/quarantine support flags,
and forbidden path classes. The `launch_context` object records that explicit
subcommands stay on the scriptable CLI path. These are contract fields only:
the command still creates no directories, writes no registry or receipt files,
and launches no TUI.

See [`manifest-validation.md`](manifest-validation.md) for the validation
contract and current trust boundaries. See
[`store-and-routing-contract.md`](store-and-routing-contract.md) for the local
store and CLI/TUI routing contract, including `rz0 store plan` and
`rz0 store status` for read-only inspection without module install planning.

## Planned module families

- tool/package updater modules;
- manager-native uninstall modules;
- Revo-style leftover scanners;
- cache cleaners;
- environment/PATH inspectors;
- system integrity/security check integrations;
- report/export modules;
- future premium or commercial modules.

## Trust model

The initial implementation does not execute optional modules. First-party
modules should later be signed and explicitly installed or enabled. This
foundation slice only verifies local SHA-256 checksums; it does not make a
network trust decision. Third-party modules are expected eventually, but only
after a hardened trust model covering signing, provenance, sandboxing,
permissions, revocation, and abuse cases.
