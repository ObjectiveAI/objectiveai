//! A request's body, as the endpoint's own type.

use serde::de::DeserializeOwned;

use super::Error;

/// The body `text` carries, decoded as the request's own type —
/// `{}` when none was given, so a request whose every member has a
/// default takes none. A body the request does not take is
/// [`Error::Body`], with the path to the member that was wrong.
pub fn body<Q: DeserializeOwned>(text: Option<&str>) -> Result<Q, Error> {
    let mut deserializer = serde_json::Deserializer::from_str(text.unwrap_or("{}"));
    serde_path_to_error::deserialize(&mut deserializer).map_err(Error::Body)
}
