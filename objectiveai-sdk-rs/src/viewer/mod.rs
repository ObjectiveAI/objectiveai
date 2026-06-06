//! Viewer-side data structures shared between the Tauri host
//! (`objectiveai-viewer`) and any tool that needs to construct or
//! parse viewer events without depending on Tauri.
//!
//! - [`Event`] — the event bus enum (Inbound + CliCommand) fanned out
//!   to JS via the host's bridge.
//! - [`EventSender`] / [`EventReceiver`] — the mpsc channel aliases
//!   the host uses to route events.

mod events;

pub use events::*;
