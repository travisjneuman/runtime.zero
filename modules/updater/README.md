# Updater module

The updater is a first-party, development-only read-only module. It maps
caller-supplied installed/manager evidence into the shared finding contract and
can bind update candidates to the foundation action-plan contract.

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
```

The action plan and serial queue are always `dry_run: true`, report
`writes_attempted: false`, and never authorize execution. Queue items are
ordered, individually identified, and designed to pause on failure, drift,
cancellation, or recovery requirements. Missing installed/manager evidence or
an exact absolute manager executable remains blocked. The module includes bounded, locale-reviewed parser slices for Homebrew JSON,
APT, DNF, Pacman, and MacPorts fixture output, plus explicit probe specifications
for Windows Winget and the major Linux/macOS managers. Locale-unsafe sources
fail closed. The module still does not discover packages, access a network, run
managers, or execute updates; live process integration and production
process/transaction gates remain separate work.
