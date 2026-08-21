//! Rust-first TUI presentation boundary.
//!
//! This module owns presentation state and rendering only. It consumes typed,
//! bounded snapshots from the foundation adapter; it does not own providers,
//! processes, confirmation, transactions, receipts, or recovery execution.

pub mod foundation_adapter;
pub mod layout;
pub mod messages;
pub mod model;
pub mod screens;
pub mod state;
pub mod terminal;
pub mod testkit;
pub mod theme;
pub mod widgets;

pub use terminal::run_interactive_tui;

/// The new frontend is opt-in while Gate B/C evidence is collected. The
/// default launch path remains the established TUI until the cutover gates in
/// the RFC are accepted.
pub fn next_frontend_requested() -> bool {
    std::env::var("RZ0_TUI_FRONTEND").is_ok_and(|value| value == "next")
}
