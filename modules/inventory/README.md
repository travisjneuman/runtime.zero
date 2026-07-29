# runtime.zero first-party inventory module

This workspace package is the first read-only feature module for
`runtime.zero`. It is source-available for development and contract validation;
it is not a published or installed module package and the `rz0` core does not
load or execute it. A small `rz0-inventory-contract` workspace crate shares the
JSON model without making this package depend on the core TUI/CLI stack.

## Development commands

From the repository root:

```bash
cargo run -p rz0-module-inventory -- --help
cargo run -p rz0-module-inventory -- --fixture modules/inventory/tests/fixtures/valid.json --format json
cargo run -p rz0-module-inventory -- --format json --redact-paths
```

The fixture path is deterministic and reads only the selected local JSON file.
The live command reads the current process PATH. On Windows it also reads User
and Machine PATH values from the standard environment registry keys using
read-only access.

Additional evidence is opt-in:

```bash
cargo run -p rz0-module-inventory -- --probe-versions --format json --redact-paths
cargo run -p rz0-module-inventory -- --include-apps --format json --redact-paths
```

`--probe-versions` executes exact known executable paths discovered from PATH
with symlink/reparse-component rejection, static version-only arguments, a
two-second timeout, bounded output, and no shell. Script-based probes remain disabled. `--include-apps` is Windows-only and
reads standard uninstall registry views; it omits raw registry key names and
does not invoke package managers or uninstallers.

## Privacy

Local paths can contain usernames or private directory names. Output is local by
default; use `--redact-paths` before sharing it. Redaction replaces path values
with stable report-local placeholders without exposing the original values.
It does not redact application names, versions, or publishers from opt-in app
inventory; review those fields separately before sharing. Hostname and
current-user identity remain omitted. Raw registry keys,
credentials, sessions, browser profiles, projects, backups, and unknown user
data are outside this module's contract.

## Boundaries

The module does not:

- write PATH or registry state;
- install, update, uninstall, repair, clean, quarantine, or restore anything;
- list package-manager catalogs or contact a network source;
- load third-party code;
- create services, tasks, persistence, or account actions;
- publish or install itself.

See [`../../docs/inventory-schema.md`](../../docs/inventory-schema.md) for the
shared output contract.
