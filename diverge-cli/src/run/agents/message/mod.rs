//! `diverge run agents message …`.
//!
//! The two things an agent is asked: a chat message, or a structured
//! decision. One file per request: the tree of files is the tree of
//! subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod chat;
pub mod structured_decisions;

/// The requests that ask an agent something.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Message {
    /// Sending an agent a chat message.
    Chat(chat::Arguments),
    /// Putting a structured decision to an agent.
    StructuredDecisions(structured_decisions::Arguments),
}

/// The request put, by its subcommand.
pub async fn run(
    command: Message,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match command {
        Message::Chat(arguments) => chat::run(arguments, target, emitter).await,
        Message::StructuredDecisions(arguments) => structured_decisions::run(arguments, target, emitter).await,
    }
}
