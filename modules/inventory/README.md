# runtime.zero first-party inventory module

This workspace package is the first read-only feature module for
`runtime.zero`. It is source-available for development and contract validation;
it is not a published or installed lifecycle package. The `rz0` core embeds its
library as the built-in read-only collector but does not execute its development
binary. A small `rz0-inventory-contract` workspace crate shares the
JSON model without making this package depend on the core TUI/CLI stack. Core
maps software records into the path-free `rz0 apps` catalog and deterministic
identity groups. Source-native bundle/package/desktop/receipt/product identifiers
take precedence over name heuristics while preserving source/version
disagreement; neither form authorizes an action.

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

- Windows reads standard uninstall registry views plus service/driver registry
  metadata and omits raw path-like key fields;
- macOS enumerates direct `.app` directories, bounded `Info.plist` metadata,
  Homebrew Cellar/Caskroom and MacPorts roots, Apple Installer receipt plists,
  and launchd plist labels without invoking a manager or `launchctl`;
- Linux parses bounded regular XDG desktop entries, direct dpkg status, pacman
  local metadata, Flatpak `active/metadata` records, and systemd unit-file labels
  without emitting/executing desktop `Exec` values or invoking a manager/service
  controller. Flatpak records are metadata evidence only and do not gain
  uninstall/update authority.

The collectors reject symlinked roots/records, cap output at 4,096 software plus
4,096 service records, and do not invoke package managers, applications,
scripts, service controllers, or uninstallers. Service records describe metadata
presence/configuration, not authoritative running state.

## Privacy

Local paths can contain usernames or private directory names. Paths are redacted
by default with stable report-local placeholders from the shared privacy
foundation. Raw local paths require the explicit `--include-raw-paths` flag and
must be reviewed before sharing.
It does not redact software names, versions, publishers, source identifiers, or
service labels; review those fields separately before sharing. Hostname and
current-user identity remain omitted. Raw registry keys,
credentials, sessions, browser profiles, projects, backups, and unknown user
data are outside this module's contract.

## Boundaries

The module does not:

- write PATH or registry state;
- install, update, uninstall, repair, clean, quarantine, or restore anything;
- invoke package managers or contact a network source;
- load third-party code;
- create services, tasks, persistence, or account actions;
- publish or install itself.

Remaining work includes full Windows/Linux runtime proof, broader in-scope
package/service/persistence sources and live status on every platform, trusted
publisher/linkage evidence, stronger product identity and
publisher evidence, final-artifact performance/privacy/accessibility coverage,
and signed lifecycle integration. See
[`../../docs/inventory-schema.md`](../../docs/inventory-schema.md) for the shared
output contract and
[`../../docs/project-status-and-resumption.md`](../../docs/project-status-and-resumption.md)
for the current maturity boundary.
