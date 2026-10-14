//! `diverge run standalone-tools connect …`.
//!
//! Serving a tool the daemon runs to another daemon: the scope that
//! daemon's agents reach the tool through, over the daemon connection,
//! as [`connect`](diverge_sdk::daemon::endpoints::standalone_tools::connect) states it. A command line holds the scope for
//! exactly one of the unary MCP exchanges it carries, and that exchange
//! is the subcommand: the tool connected to, the exchange put, its
//! result printed, and the scope disconnected. The notifications
//! stream is no exchange of a command line's and is not among them.
//! Beneath each is `request-schema`: the JSON Schema of the connect
//! request, which is the body every one of them takes.
//! One file per exchange: the tree of files is the tree of
//! subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod call_tool;
pub mod list_resources;
pub mod list_tools;
pub mod read_resource;

/// The unary MCP exchanges a connected tool answers, one per
/// subcommand.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Connect {
    /// One call of one of the tool's MCP tools, by name.
    CallTool(call_tool::Arguments),
    /// The tool's MCP resources, one page.
    ListResources(list_resources::Arguments),
    /// The tool's MCP tools, one page.
    ListTools(list_tools::Arguments),
    /// One read of one of the tool's MCP resources, by URI.
    ReadResource(read_resource::Arguments),
}

/// The exchange put, by its subcommand.
pub async fn run(
    command: Connect,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match command {
        Connect::CallTool(arguments) => call_tool::run(arguments, target, emitter).await,
        Connect::ListResources(arguments) => list_resources::run(arguments, target, emitter).await,
        Connect::ListTools(arguments) => list_tools::run(arguments, target, emitter).await,
        Connect::ReadResource(arguments) => read_resource::run(arguments, target, emitter).await,
    }
}
