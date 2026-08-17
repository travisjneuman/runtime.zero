# Foundation Configuration Contract

`crates/configuration-contract/` owns the schema-1 configuration baseline. The
first schema intentionally accepts only compiled, fail-closed defaults; it does
not load user files, environment overrides, registry values, remote policy, or
module-owned settings.

The canonical configuration requires:

- report-local path redaction and no hostname, current-user, environment-value,
  or telemetry collection;
- disabled production modules, remote execution, and shell execution;
- network default-deny;
- one concurrent module process and the shared schema-1 process ceilings;
- dry-run, exact confirmation, and quarantine before removal;
- no automatic retry, update, background service, implicit migration, or startup
  repair;
- `configuration_authorizes_execution: false`.

Unknown fields, oversized documents, any permissive drift, or a source other
than built-in defaults fail closed. Compact canonical JSON ends in LF and has a
domain-stable SHA-256 used by foundation diagnostics. Configuration cannot grant
a capability or authorize execution, mutation, recovery, or release.

Future configurable values require a new reviewed schema with explicit
migration and precedence rules. Modules may receive only effective policy views
from the foundation and may narrow ceilings; they must not parse private config
files or expand foundation policy.

## End-state module configuration

The schema-1 default is deliberately foundation-only, but the product direction
requires user-selectable module configuration. A future reviewed schema must
store module settings under the foundation-owned module state, bind the
effective configuration digest into diagnostics and action plans, and preserve
module settings across disable/enable unless the user explicitly resets them.

Each module configuration must declare its schema, defaults, sensitive fields,
allowed values, migration path, resource/network implications, and whether a
change requires deactivation or fresh confirmation. Configuration can narrow a
module's behavior; it can never grant a capability, enable a provider, authorize
an external write, or bypass a platform gate. Disabling a module must stop its
collection, scheduling, network activity, UI actions, and mutation while
retaining settings and receipts. Configuration reset and module uninstall are
separate explicit operations with their own data-retention and recovery review.

The target TUI and CLI will expose module-specific settings through shared
foundation controls. They do not exist in the current command surface and must
not be represented by ad hoc environment variables or module-private policy
files.
