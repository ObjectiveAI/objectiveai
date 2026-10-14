//! `diverge run standalone-tools connect read-resource`: one read of
//! one of the connected tool's MCP resources, through the scope
//! [`connect`](diverge_sdk::daemon::endpoints::standalone_tools::connect) states.

use clap::Args;
use diverge_sdk::daemon::endpoints::standalone_tools::connect::client::request;
use diverge_sdk::wire::client::handle::Handle;
use rmcp::model::ReadResourceRequestParams;

use crate::run::{Emitter, Error, body, shape};

/// One read of one of the tool's MCP resources, by URI.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct Arguments {
    /// What the connect carries, as a JSON object: see
    /// [`request::Frame`]. Absent, `{}`.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// The resource to read, by its URI in the tool's list.
    #[arg(long, value_name = "URI")]
    pub uri: String,
}

/// The tool connected to, the resource read, its contents printed as
/// the MCP result's own JSON, and the scope disconnected.
pub async fn run(
    arguments: Arguments,
    handle: &Handle,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    let request: request::Frame = body(arguments.body.as_deref())?;
    let params = ReadResourceRequestParams::new(arguments.uri);
    shape::mcp(
        handle,
        &request,
        async |tool| tool.read_resource(params).await,
        emitter,
    )
    .await
}
