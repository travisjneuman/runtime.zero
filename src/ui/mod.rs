//! Rust-first TUI presentation boundary.
//!
//! This module owns presentation state and rendering only. It consumes typed,
//! bounded snapshots from the foundation adapter; it does not own providers,
//! processes, confirmation, transactions, receipts, or recovery execution.

pub mod foundation_adapter;
pub mod layout;
pub mod messages;
pub mod model;
pub mod state;
pub mod task_terminal;
pub mod testkit;
pub mod text;
pub mod theme;
pub mod widgets;

pub use task_terminal::run_interactive_tui;
