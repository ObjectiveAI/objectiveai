//! The `relay` member found in a request's bytes: its first name taken
//! off.

use serde_json::Value;

/// The first name of the relay the request carries, if it carries one,
/// and the request's bytes for the next daemon: the tag kept, the JSON
/// after it read as an object, the first name taken off `relay` — the
/// rest left as `relay`, or `relay` taken out where none is left — and
/// the object written again. `None` when there is no `relay`, or it is
/// `null` or empty, or the bytes are not a tag and a JSON object at all
/// — which is then the handler's to refuse, as it would have been — or
/// `relay` is not an array of names.
pub fn peel(payload: &[u8]) -> Option<(String, Vec<u8>)> {
    let (tag, rest) = payload.split_first()?;
    let mut body: Value = serde_json::from_slice(rest).ok()?;
    let object = body.as_object_mut()?;
    let relay = object.remove("relay")?;
    if relay.is_null() {
        return None;
    }
    let mut names: Vec<String> = serde_json::from_value(relay).ok()?;
    if names.is_empty() {
        return None;
    }
    let first = names.remove(0);
    if !names.is_empty() {
        object.insert("relay".to_string(), Value::from(names));
    }
    let mut bytes = vec![*tag];
    serde_json::to_writer(&mut bytes, &body).ok()?;
    Some((first, bytes))
}
