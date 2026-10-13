//! What is put: the request by name, with its body.

use diverge_sdk::daemon::endpoints;
use serde_json::Value;

use super::Error;

/// One request as the wire carries it: its name, its tag, and its
/// bytes — the tag, then the body's JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The name, as given: `agents/create`.
    pub path: String,
    /// The tag the name is.
    pub tag: u8,
    /// The body, as JSON: `{}` when none was given.
    pub body: Value,
    /// The payload: the tag, then the body.
    pub bytes: Vec<u8>,
}

impl Request {
    /// The request `path` names with `body`: a name that is no
    /// request is [`Error::NoSuchRequest`]; a body that is not a JSON
    /// object is [`Error::Body`]. Whether the body is what the request
    /// takes is asked where the request is decoded as its own type.
    pub fn new(path: &str, body: Option<&str>) -> Result<Request, Error> {
        let Some(tag) = endpoints::tag(path) else {
            return Err(Error::NoSuchRequest(path.to_string()));
        };
        let body: Value = match body {
            Some(text) => serde_json::from_str(text).map_err(Error::Body)?,
            None => Value::Object(Default::default()),
        };
        if !body.is_object() {
            return Err(Error::NotAnObject);
        }
        let mut bytes = vec![tag];
        serde_json::to_writer(&mut bytes, &body).map_err(Error::Body)?;
        Ok(Request {
            path: path.to_string(),
            tag,
            body,
            bytes,
        })
    }

    /// Refuse a request that would never end: `agents/logs` with
    /// `watch` true, which keeps its scope open for every item to
    /// come. [`Error::Endless`].
    pub fn refuse_endless(&self) -> Result<(), Error> {
        if self.path == "agents/logs" && self.body.get("watch").and_then(Value::as_bool) == Some(true) {
            return Err(Error::Endless);
        }
        Ok(())
    }
}
