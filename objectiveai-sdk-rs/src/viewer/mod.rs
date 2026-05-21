//! Viewer-side data structures shared between the Tauri host
//! (`objectiveai-viewer`) and any tool that needs to construct or
//! parse viewer events without depending on Tauri.
//!
//! - [`Event`] — the event bus enum (Inbound + CliCommand) fanned out
//!   to JS via the host's bridge.
//! - [`EventSender`] / [`EventReceiver`] — the mpsc channel aliases
//!   the host uses to route events.
//! - [`cli_event_sink`] (cli feature) — convenience adapter that
//!   produces a [`Handle`](crate::cli::output::Handle) for an
//!   `objectiveai_cli::run` invocation and forwards each emitted line
//!   onto a given [`EventSender`] as [`Event::CliCommand`].

mod events;

pub use events::*;

#[cfg(feature = "cli")]
mod cli;
#[cfg(feature = "cli")]
pub use cli::*;
