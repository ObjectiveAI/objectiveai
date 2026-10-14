//! `diverge run standalone-tools connect list-resources`: the
//! connected tool's MCP resources, one page, through the scope
//! [`connect`](diverge_sdk::daemon::endpoints::standalone_tools::connect) states.

use clap::Args;
use diverge_sdk::daemon::endpoints::standalone_tools::connect::client::request;
use rmcp::model::PaginatedRequestParams;

use crate::run::{Below, Emitter, Error, Target, body, request_schema, shape};

/// The tool's MCP resources, one page.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
#[command(subcommand_negates_reqs = true, args_conflicts_with_subcommands = true)]
pub struct Arguments {
    /// What the connect carries, as a JSON object: see
    /// [`request::Frame`]. Absent, `{}`.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// The page to list, as the previous page's result named it.
    /// Absent, the first.
    #[arg(long, value_name = "CURSOR")]
    pub cursor: Option<String>,
    /// What is beneath the exchange: `request-schema`, the JSON Schema
    /// of the connect request every exchange takes as its body, printed
    /// in place of connecting.
    #[command(subcommand)]
    pub below: Option<Below>,
}

/// The tool connected to, its resources listed from the cursor, the
/// page printed as the MCP result's own JSON, and the scope
/// disconnected.
pub async fn run(
    arguments: Arguments,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    if let Some(Below::RequestSchema(_)) = arguments.below {
        return request_schema::<request::Frame>("diverge run standalone-tools connect", emitter)
            .await;
    }
    let request: request::Frame = body(arguments.body.as_deref())?;
    let handle = target.dial().await?;
    let params = arguments
        .cursor
        .map(|cursor| PaginatedRequestParams::default().with_cursor(Some(cursor)));
    shape::mcp(
        &handle,
        &request,
        async |tool| tool.list_resources(params).await,
        emitter,
    )
    .await
}
