//! `diverge run standalone-tools connect read-resource`: one read of
//! one of the connected tool's MCP resources, through the scope
//! [`connect`](diverge_sdk::daemon::endpoints::standalone_tools::connect) states.

use clap::Args;
use diverge_sdk::daemon::endpoints::standalone_tools::connect::client::request;
use rmcp::model::{ReadResourceRequestParams, ReadResourceResult};

use crate::run::{Below, Emitter, Error, Target, body, request_schema, response_schema, shape};

/// One read of one of the tool's MCP resources, by URI.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
#[command(subcommand_negates_reqs = true, args_conflicts_with_subcommands = true)]
pub struct Arguments {
    /// What the connect carries, as a JSON object: see
    /// [`request::Frame`]. Absent, `{}`.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// The resource to read, by its URI in the tool's list.
    #[arg(long, value_name = "URI")]
    pub uri: String,
    /// What is beneath the exchange: `request-schema`, the JSON Schema
    /// of the connect request every exchange takes as its body, or
    /// `response-schema`, the JSON Schema of every line it prints —
    /// printed in place of connecting.
    #[command(subcommand)]
    pub below: Option<Below>,
}

/// The tool connected to, the resource read, its contents printed as
/// the MCP result's own JSON, and the scope disconnected.
pub async fn run(
    arguments: Arguments,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match arguments.below {
        Some(Below::RequestSchema(_)) => {
            return request_schema::<request::Frame>(
                "diverge run standalone-tools connect",
                emitter,
            )
            .await;
        }
        Some(Below::ResponseSchema(_)) => {
            return response_schema::<shape::mcp::Line<ReadResourceResult>>(
                "diverge run standalone-tools connect read-resource",
                emitter,
            )
            .await;
        }
        None => {}
    }
    let request: request::Frame = body(arguments.body.as_deref())?;
    let handle = target.dial().await?;
    let params = ReadResourceRequestParams::new(arguments.uri);
    shape::mcp(
        &handle,
        &request,
        async |tool| tool.read_resource(params).await,
        emitter,
    )
    .await
}
