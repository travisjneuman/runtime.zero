# runtime.zero TUI redesign archive

Status: superseded by the Rust-first Dossier Queue implementation.

This file preserves the pointer for older task-first TUI design notes. The
current product contract is [`tui.md`](tui.md), and the reviewable architecture
and migration record is [`tui-front-end-reset-rfc.md`](tui-front-end-reset-rfc.md).

The active TUI has five stable destinations—Overview, Explore, Review, Activity,
and Modules—and consumes the typed foundation evidence/action contracts from
`src/ui/`. The former dashboard, command rail, and six-section workspace model
are not current implementation or navigation contracts.

The safety boundary remains unchanged: presentation code owns navigation,
focus, layout, rendering, and terminal restoration; foundation code owns
evidence, provider identity, module lifecycle, action plans, confirmation,
cancellation, process hosting, transactions, receipts, verification, and
recovery. See [`SAFETY.md`](../SAFETY.md) and the active TUI guide for the
current behavior and open human/platform acceptance lanes.
