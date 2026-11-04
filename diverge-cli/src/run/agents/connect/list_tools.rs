//! `diverge run agents connect list-tools`: the
//! MCP tools an agent sees, one page, through the scope
//! [`connect`](diverge_sdk::daemon::endpoints::agents::connect) states.

use clap::Args;
use diverge_sdk::daemon::endpoints::agents::connect::client::request;
use rmcp::model::{ListToolsResult, PaginatedRequestParams};

use crate::run::{Below, Emitter, Error, Target, body, request_schema, response_schema, shape};

/// The MCP tools the agent sees, one page.
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
    /// of the connect request every exchange takes as its body, or
    /// `response-schema`, the JSON Schema of every line it prints —
    /// printed in place of connecting.
    #[command(subcommand)]
    pub below: Option<Below>,
}

/// The agent connected to, its tools listed from the cursor, the
/// page printed as the MCP result's own JSON, and the scope
/// disconnected.
pub async fn run(
    arguments: Arguments,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match arguments.below {
        Some(Below::RequestSchema(_)) => {
            return request_schema::<request::Frame>(
                "diverge run agents connect",
                emitter,
            )
            .await;
        }
        Some(Below::ResponseSchema(_)) => {
            return response_schema::<shape::mcp::Line<ListToolsResult>>(
                "diverge run agents connect list-tools",
                emitter,
            )
            .await;
        }
        None => {}
    }
    let request: request::Frame = body(arguments.body.as_deref())?;
    let handle = target.dial().await?;
    let params = arguments
        .cursor
        .map(|cursor| PaginatedRequestParams::default().with_cursor(Some(cursor)));
    shape::mcp::agent(
        &handle,
        &request,
        async |agent| agent.list_tools(params).await,
        emitter,
    )
    .await
}
