//! `diverge run standalone-tools connect list-tools`: the
//! connected tool's MCP tools, one page, through the scope
//! [`connect`](diverge_sdk::daemon::endpoints::standalone_tools::connect) states.

use clap::Args;
use diverge_sdk::daemon::endpoints::standalone_tools::connect::client::request;
use diverge_sdk::wire::client::handle::Handle;
use rmcp::model::PaginatedRequestParams;

use crate::run::{Emitter, Error, body, shape};

/// The tool's MCP tools, one page.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct Arguments {
    /// What the connect carries, as a JSON object: see
    /// [`request::Frame`]. Absent, `{}`.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// The page to list, as the previous page's result named it.
    /// Absent, the first.
    #[arg(long, value_name = "CURSOR")]
    pub cursor: Option<String>,
}

/// The tool connected to, its tools listed from the cursor, the
/// page printed as the MCP result's own JSON, and the scope
/// disconnected.
pub async fn run(
    arguments: Arguments,
    handle: &Handle,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    let request: request::Frame = body(arguments.body.as_deref())?;
    let params = arguments
        .cursor
        .map(|cursor| PaginatedRequestParams::default().with_cursor(Some(cursor)));
    shape::mcp(
        handle,
        &request,
        async |tool| tool.list_tools(params).await,
        emitter,
    )
    .await
}
