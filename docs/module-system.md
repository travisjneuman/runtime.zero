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
```

The JSON output uses schema version `1` and separates:

- `core`;
- `installed_modules`;
- `planned_module_families`.

An empty `installed_modules` list is valid and expected for the foundation-only build.

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

The initial implementation does not execute optional modules. First-party modules should later be signed/checksummed and explicitly installed or enabled. Third-party modules are expected eventually, but only after a hardened trust model covering signing, provenance, sandboxing, permissions, revocation, and abuse cases.
