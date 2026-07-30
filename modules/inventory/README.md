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
cargo run -p rz0-module-inventory -- --format json
```

The fixture path is deterministic and reads only the selected local JSON file.
The live command reads the current process PATH. On Windows it also reads User
and Machine PATH values from the standard environment registry keys using
read-only access.

Additional evidence is opt-in:

```bash
cargo run -p rz0-module-inventory -- --probe-versions --format json
cargo run -p rz0-module-inventory -- --include-apps --format json
```

`--probe-versions` executes exact known executable paths discovered from PATH
with symlink/reparse-component rejection, static version-only arguments, a
two-second timeout, bounded output, and no shell. Script-based probes remain
disabled. `--include-apps` is available on supported platforms and remains
explicit:

- Windows reads standard uninstall registry views and omits raw key names;
- macOS enumerates only direct `.app` directories under known system/user roots
  and does not open bundle contents, so version and publisher remain unknown;
- Linux parses only regular XDG `.desktop` files up to 64 KiB, honors user-root
  and `Hidden=true` precedence, emits only `Type=Application` names/paths, and
  never emits or executes `Exec` values.

The collectors reject symlinked roots/records, cap entry inspection and output at 4,096 applications,
and do not invoke package managers, applications, scripts, or uninstallers.

## Privacy

Local paths can contain usernames or private directory names. Paths are redacted
by default with stable report-local placeholders from the shared privacy
foundation. Raw local paths require the explicit `--include-raw-paths` flag and
must be reviewed before sharing.
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
