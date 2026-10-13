//! A connect, one MCP exchange through it, and the disconnect.

use diverge_sdk::daemon::endpoints::standalone_tools::connect::client::execute::{self, ExecuteError};
use diverge_sdk::daemon::endpoints::standalone_tools::connect::client::request;
use diverge_sdk::wire::client::handle::Handle;
use rmcp::model::{CallToolRequestParams, PaginatedRequestParams, ReadResourceRequestParams};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::request as decode;
use crate::arguments::Mcp;
use crate::run::{Emitter, Error};

/// The tool connected to, the one exchange `mcp` names put with the
/// params `request` carries, the result printed as the MCP result's
/// own JSON, and the scope disconnected and waited to its end. A
/// connect the daemon refuses — no such tool, forbidden, or its own
/// error — is printed as the answer it is, and the run ends there.
pub async fn mcp(handle: &Handle, bytes: &[u8], mcp: Mcp, request: Option<&str>, emitter: &mut Emitter) -> Result<(), Error> {
    let connect: request::Frame = decode(bytes)?;
    let tool = match execute::execute(handle, &connect).await {
        Ok(tool) => tool,
        Err(ExecuteError::NotFound) => return emitter.emit_value(Value::from("not_found")).await,
        Err(ExecuteError::Forbidden) => return emitter.emit_value(Value::from("forbidden")).await,
        Err(ExecuteError::Daemon(error)) => return emitter.emit_value(serde_json::json!({ "error": error })).await,
        Err(ExecuteError::Closed | ExecuteError::Send(_)) => return Err(Error::Rejected),
        Err(error) => return Err(Error::Exchange(error.to_string())),
    };
    let outcome = match mcp {
        Mcp::ListTools => answer(tool.list_tools(optional::<PaginatedRequestParams>(request)?).await),
        Mcp::ListResources => answer(tool.list_resources(optional::<PaginatedRequestParams>(request)?).await),
        Mcp::CallTool => answer(tool.call_tool(required::<CallToolRequestParams>(request)?).await),
        Mcp::ReadResource => answer(tool.read_resource(required::<ReadResourceRequestParams>(request)?).await),
    };
    let _ = tool.disconnect().await;
    tool.wait().await;
    emitter.emit_value(outcome?).await
}

/// The exchange's params, from `--mcp-request`, for a method that
/// takes none: decoded as the method's own type when given, `None`
/// when not.
fn optional<T: DeserializeOwned>(request: Option<&str>) -> Result<Option<T>, Error> {
    request.map(|text| serde_json::from_str(text).map_err(Error::McpRequest)).transpose()
}

/// The exchange's params, from `--mcp-request`, for a method that
/// takes one: decoded as the method's own type; not given is
/// [`Error::McpRequestMissing`].
fn required<T: DeserializeOwned>(request: Option<&str>) -> Result<T, Error> {
    match request {
        Some(text) => serde_json::from_str(text).map_err(Error::McpRequest),
        None => Err(Error::McpRequestMissing),
    }
}

/// The method's result as JSON, or its failure in the SDK's words.
fn answer<T: Serialize, E: std::fmt::Display>(outcome: Result<T, E>) -> Result<Value, Error> {
    let result = outcome.map_err(|error| Error::Exchange(error.to_string()))?;
    serde_json::to_value(result).map_err(Error::Json)
}
