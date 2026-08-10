# Updater module

The updater is a first-party module. The standalone binary maps
caller-supplied installed/manager evidence into the shared finding contract and
can bind update candidates to the foundation action-plan contract. The core
owns the manager execution lane so the module cannot invent its own process,
confirmation, or transaction stack.

The standalone binary accepts bounded JSON on standard input:

```bash
rz0-updater --format json < updater-finding-input.json
rz0-updater --plan --format json < updater-finding-input.json
rz0-updater --plan --queue --format json < updater-finding-input.json
```

The core can also parse a locally captured manager response without invoking the
manager:

```bash
rz0 updates --dry-run --manager homebrew-formula \
  --manager-output /tmp/homebrew-outdated.json \
  --executable /opt/homebrew/bin/brew --plan --queue --format json

# After reviewing the challenge emitted by --apply without --confirm:
rz0 updates --apply --probe --manager homebrew-formula \
  --executable /opt/homebrew/bin/brew --allow-network-read \
  --allow-network-write --action <exact-action-id> \
  --accept-no-rollback --challenge-issued-unix-seconds <issued> \
  --confirm '<exact-phrase>'
```

The action plan and serial queue remain `dry_run: true` review artifacts with
`writes_attempted: false` and `product_execution_authorized: false`. Queue items
are ordered, individually identified, and pause on failure, drift, cancellation,
or recovery requirements. Missing installed/manager evidence or an exact
absolute manager executable remains blocked. The core additionally exposes
explicit `rz0 updates --apply` and interactive serial `--all` lanes. They require
live evidence, an allowlisted manager path, explicit network-write approval,
an initialized private store, an exact short-lived confirmation, and a manual-
recovery acknowledgement when rollback is not proven. The direct manager
process is bounded, journaled, receipt-backed, and followed by fresh evidence
verification; no sudo/helper or arbitrary shell path is used.

The module includes bounded, locale-reviewed parser slices for Homebrew JSON,
APT, DNF, Pacman, and MacPorts fixture output, plus explicit probe
specifications for Windows Winget and major Linux/macOS managers. Winget,
Zypper, Snap, and Flatpak parsing currently fails closed as not yet locale-safe.
A probe specification is not production runtime evidence.

## Apply-lane limitations

The core apply lane is pre-alpha rather than a supported module lifecycle:

- it allowlists an absolute manager path but does not yet bind the
  opened-artifact lease through the actual spawn;
- network read/write flags record explicit intent but do not create an OS
  network sandbox;
- Unix process groups and bounded output are not a capability sandbox;
- Windows production execution fails closed;
- native rollback, complete cancellation, and platform power-loss recovery are
  missing;
- updater-specific receipt publication still needs integration with the full
  canonical commit/recovery coordinator;
- no real update on a normal host is release evidence.

Third-party module execution, uninstall/cleanup mutation, module lifecycle, and
release support remain separate foundation gates. See
[`../../docs/project-status-and-resumption.md`](../../docs/project-status-and-resumption.md)
and [`../../docs/completion-checklist.md`](../../docs/completion-checklist.md).
