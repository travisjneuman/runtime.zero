# Updater module

The updater is a first-party, development-only read-only module. It maps
caller-supplied installed/manager evidence into the shared finding contract and
can bind update candidates to the foundation action-plan contract.

The standalone binary accepts bounded JSON on standard input:

```bash
rz0-updater --format json < updater-finding-input.json
rz0-updater --plan --format json < updater-finding-input.json
```

The action plan is always `dry_run: true`, reports `writes_attempted: false`,
and never authorizes execution. Missing installed/manager evidence or an exact
absolute manager executable remains blocked. The module does not discover
packages, access a network, run managers, or execute updates; live platform
adapters and the production process/transaction gates remain separate work.
