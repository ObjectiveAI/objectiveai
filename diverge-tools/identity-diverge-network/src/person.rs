//! Who is at the page.

use axum::http::HeaderMap;
use diverge_sdk::shared::mcp::attestation::Caller;

use crate::failure::Failure;

/// Whether a page's request is a person's, by the `diverge.network`
/// header the daemon stamped it with. A request with no stamp, one
/// that does not read, and one stamped as an agent, is refused: the
/// page is where a person signs an account of theirs in, and an agent
/// has the MCP tools.
///
/// Which person is not asked: the accounts are the tool's, shared by
/// everyone the daemon lets open its page.
pub fn person(headers: &HeaderMap) -> Result<(), Failure> {
    let stamp = headers.get(diverge_sdk::shared::mcp::KEY).ok_or_else(|| Failure::Person("the request carries no stamp".to_string()))?;
    let text = stamp.to_str().map_err(|error| Failure::Person(error.to_string()))?;
    match serde_json::from_str::<Caller>(text) {
        Ok(Caller::Client { .. }) => Ok(()),
        Ok(Caller::Agent(_)) => Err(Failure::Person("an agent, not a person".to_string())),
        Err(error) => Err(Failure::Person(error.to_string())),
    }
}
