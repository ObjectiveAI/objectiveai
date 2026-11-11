//! Who is at the page: the person behind a frontend request, as the
//! daemon attests it, never as the request says.

use axum::http::HeaderMap;
use diverge_sdk::shared::Identity;
use diverge_sdk::shared::mcp::attestation::Caller;

/// The person behind a page's request, from the `diverge.network`
/// header the daemon stamped it with: the account. A request with no
/// stamp, one that does not read, or one stamped as an agent, is
/// refused, with why.
pub fn person(headers: &HeaderMap) -> Result<Identity, String> {
    let stamp = headers.get(diverge_sdk::shared::mcp::KEY).ok_or_else(|| "the request carries no stamp".to_string())?;
    let text = stamp.to_str().map_err(|error| error.to_string())?;
    match serde_json::from_str::<Caller>(text) {
        Ok(Caller::Client { identity, .. }) => Ok(identity),
        Ok(Caller::Agent(_)) => Err("an agent, not a person".to_string()),
        Err(error) => Err(error.to_string()),
    }
}
