# runtime.zero TUI redesign archive

Status: superseded by the task-first operator-console rebuild.

This file preserves the pointer for older task-first TUI design notes. The
current product contract is [`tui.md`](tui.md), and the reviewable architecture
and migration record is [`tui-front-end-reset-rfc.md`](tui-front-end-reset-rfc.md).

The active TUI is a single task path: Home, Inventory, Evidence, Plan Review,
dedicated Confirmation, and Activity. Inventory contains module-contributed
evidence; modules do not become navigation tabs. The former dashboard, command
rail, numeric route shortcuts, and packed five-destination shell are not
current implementation or navigation contracts. See [`tui.md`](tui.md) for the
actual controls and state vocabulary.

The safety boundary remains unchanged: presentation code owns navigation,
focus, layout, rendering, and terminal restoration; foundation code owns
evidence, provider identity, module lifecycle, action plans, confirmation,
cancellation, process hosting, transactions, receipts, verification, and
recovery. See [`SAFETY.md`](../SAFETY.md) and the active TUI guide for the
current behavior and open human/platform acceptance lanes.
