# Domain Finding Classifier Modules

The five source packages below contain only family-specific classification over
caller-supplied synthetic evidence:

- `modules/updater/` — installed plus manager-owned update candidates;
- `modules/uninstall/` — installed manager-record uninstall candidates;
- `modules/leftovers/` — exact runtime-owned orphan/executable quarantine
  candidates with protected/user/system/unknown evidence blocked;
- `modules/cache/` — exact runtime-owned cache quarantine candidates while
  manager/system/user evidence stays report-only and unknown ownership blocks;
- `modules/security-integrity/` — exact digest match/mismatch observations that
  remain report-only; unknown ownership blocks.

Each package maps its strict typed input into `crates/finding-contract/`. The
foundation owns producer/category binding, privacy, protected-data policy,
resource ceilings, evidence identity, sorting, summary counts, and authority
refusal. Modules do not duplicate those controls.

These are source-level classifiers, not complete modules. They have no live
Windows/macOS/Linux discovery adapter, binary/process protocol, package-manager
or filesystem access, network access, action-plan generation, TUI flow, signed
artifact, installation/activation path, transaction execution, rollback, or
production package. Their manifests remain `planned` with no host permission.
Core does not install or execute them.

The current tests establish only deterministic synthetic classification:

- missing installed/manager ownership cannot become updater/uninstall action
  candidates;
- only exact runtime-owned leftover/cache evidence can become a quarantine
  candidate;
- protected and unknown evidence stays blocked;
- integrity mismatch is high-risk report evidence, never remediation authority.

Before any family advances, add bounded adversarial fixture sets, exact input
provenance, live platform adapters behind explicit read capabilities, finding-
bound dry-run plans where applicable, final-artifact protocol/lifecycle proof,
and every Windows/macOS/Linux release-ledger cell. Mutating behavior still
requires confirmation, transaction, rollback, cancellation, isolation, and
runtime evidence from the shared foundation.
