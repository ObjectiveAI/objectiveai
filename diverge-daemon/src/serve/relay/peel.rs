//! The `relay` member found in a request's bytes, and taken out.

use diverge_sdk::shared::relay::Relay;
use serde_json::Value;

/// The relay the request carries, if it carries one, and the request's
/// bytes without it: the tag kept, the JSON after it read as an
/// object, `relay` taken out and the object written again. `None`
/// when there is no `relay`, or it is `null`, or the bytes are not a
/// tag and a JSON object at all — which is then the handler's to
/// refuse, as it would have been — or `relay` is not a [`Relay`].
pub fn peel(payload: &[u8]) -> Option<(Relay, Vec<u8>)> {
    let (tag, rest) = payload.split_first()?;
    let mut body: Value = serde_json::from_slice(rest).ok()?;
    let object = body.as_object_mut()?;
    let relay = object.remove("relay")?;
    if relay.is_null() {
        return None;
    }
    let relay: Relay = serde_json::from_value(relay).ok()?;
    let mut bytes = vec![*tag];
    serde_json::to_writer(&mut bytes, &body).ok()?;
    Some((relay, bytes))
}
