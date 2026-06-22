# Module System

Modules are the unit of growth for `runtime.zero`.

A module should eventually declare:

- id and display name;
- supported platforms;
- required privileges;
- discovery rules;
- update strategy;
- uninstall strategy;
- leftover scan rules;
- cleanup risk categories;
- quarantine/restore behavior;
- dry-run behavior;
- test fixtures.

## Design rule

Every module must be safe to run in discovery/dry-run mode before it is allowed to mutate anything.

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

The initial implementation will use built-in local modules only. If third-party modules are supported later, they will need explicit trust metadata, source provenance, and clear licensing.
