//! What is beneath every request: its schemas.

use clap::Subcommand;

use super::{RequestSchema, ResponseSchema};

/// The subcommands beneath every request. Given, the request is not
/// put and the daemon is not dialled: the subcommand answers from the
/// SDK alone, and whatever the leaf's own arguments require is waived.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Below {
    /// The JSON Schema of the body the request takes, as the SDK
    /// states it: one compact line, through `--jq` when one is given.
    RequestSchema(RequestSchema),
    /// The JSON Schema of every line the request can print — each
    /// answer, item, frame and error, and what the command itself
    /// prints — as one schema: one compact line, through `--jq` when
    /// one is given.
    ResponseSchema(ResponseSchema),
}
