//! A request's body, and any other JSON the command line carries, as
//! its own type.

use serde::de::DeserializeOwned;

use super::Error;

/// The body `text` carries, decoded as the request's own type —
/// `{}` when none was given, so a request whose every member has a
/// default takes none. A body the request does not take is
/// [`Error::Body`], with the path to the member that was wrong.
pub fn body<Q: DeserializeOwned>(text: Option<&str>) -> Result<Q, Error> {
    json(text.unwrap_or("{}")).map_err(Error::Body)
}

/// `text` decoded as `T`, the failure naming the path to the member
/// that was wrong: what [`body`] is made of, for the other JSON a leaf
/// takes.
pub fn json<T: DeserializeOwned>(
    text: &str,
) -> Result<T, serde_path_to_error::Error<serde_json::Error>> {
    let mut deserializer = serde_json::Deserializer::from_str(text);
    serde_path_to_error::deserialize(&mut deserializer)
}
