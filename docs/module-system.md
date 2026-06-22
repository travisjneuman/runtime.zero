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

See [`manifest-validation.md`](manifest-validation.md) for the validation
contract and current trust boundaries.

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
