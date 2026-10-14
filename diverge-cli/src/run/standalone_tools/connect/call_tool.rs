//! `diverge run standalone-tools connect call-tool`: one call of one of
//! the connected tool's MCP tools, through the scope
//! [`connect`](diverge_sdk::daemon::endpoints::standalone_tools::connect) states.

use clap::Args;
use diverge_sdk::daemon::endpoints::standalone_tools::connect::client::request;
use rmcp::model::{CallToolRequestParams, JsonObject};

use crate::run::{Below, Emitter, Error, Target, body, json, request_schema, shape};

/// One call of one of the tool's MCP tools, by name.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
#[command(subcommand_negates_reqs = true, args_conflicts_with_subcommands = true)]
pub struct Arguments {
    /// What the connect carries, as a JSON object: see
    /// [`request::Frame`]. Absent, `{}`.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// The MCP tool to call, by its name in the tool's list.
    #[arg(long, value_name = "NAME")]
    pub name: String,
    /// The call's arguments, as a JSON object — the MCP tool's input
    /// as its schema names it. Absent, none.
    #[arg(long, value_name = "JSON")]
    pub arguments: Option<String>,
    /// What is beneath the exchange: `request-schema`, the JSON Schema
    /// of the connect request every exchange takes as its body, printed
    /// in place of connecting.
    #[command(subcommand)]
    pub below: Option<Below>,
}

/// The tool connected to, the MCP tool called with the arguments, the
/// result printed as the MCP result's own JSON, and the scope
/// disconnected. Arguments that are not a JSON object are
/// [`Error::Arguments`].
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
    let mut params = CallToolRequestParams::new(arguments.name);
    if let Some(text) = arguments.arguments.as_deref() {
        params = params.with_arguments(json::<JsonObject>(text).map_err(Error::Arguments)?);
    }
    shape::mcp(
        &handle,
        &request,
        async |tool| tool.call_tool(params).await,
        emitter,
    )
    .await
}
