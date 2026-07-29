# Contributing

`runtime.zero` is public from the beginning, but external code contributions are not broadly accepted during the earliest safety-design phase.

Issues, design feedback, documentation suggestions, and safety reviews are welcome. Substantial code contributions may be deferred until the contribution/license policy is finalized.

## Early contribution rules

- Do not submit code that performs destructive cleanup, persistence, credential access, evasion, or account actions.
- Keep modules report-first and dry-run-first.
- Do not add GitHub Actions, scheduled jobs, deployment automation, package publishing, or external service writes without maintainer approval.
- Keep user data and secrets out of examples, tests, issues, and docs. Inventory
  fixtures must use synthetic paths, app names, publishers, and versions.
- Keep feature modules separate from the core and preserve fixture-first,
  read-only behavior before proposing platform probes.
- Run `cargo fmt --check`, `cargo test --workspace`, and workspace Clippy with
  warnings denied for Rust changes. Run the committed `cargo deny check` policy
  when changing dependencies or preparing a release candidate.

The repository currently uses Apache-2.0. Premium or commercial modules may use separate licenses later.
