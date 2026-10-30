//! Which MCP session an exchange toward a tool container belongs to.
//!
//! A tool container's proxy holds one MCP session to the container's
//! server per caller: each opened by an `initialize` whose `_meta`
//! carries the caller under [`KEY`](super::KEY), so the server knows
//! from the session's first message who is on the other end, and
//! whatever the server sends on a session — a notification — reaches
//! that caller alone. So every exchange the caller's side sends toward
//! a tool container names its session: the
//! [`Caller`], as the daemon attests it.
//!
//! The rules:
//!
//! 1. A [`Sessioned`] request is one JSON object:
//!    `{"session": <the caller>, "params": <the request's own JSON>}`,
//!    `params` absent when the request has no bytes of its own — the
//!    notifications request.
//! 2. Two callers are one session when their [`key`]s are one: an
//!    agent by its template and its index, a client by its issuer and
//!    its subject. The image and the name are not the session's, so an
//!    agent renamed keeps its session.
//! 3. Only the exchanges toward a tool container carry a session. An
//!    agent container's own asks, which reach the daemon and not a
//!    tool, carry the bare request as before.

use serde::Deserialize;

use super::attestation::Caller;
use super::{call_tool, list_resources, list_tools, notifications, read_resource};
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// A request with the session it belongs to.
#[derive(Debug, Clone, PartialEq)]
pub struct Sessioned<R> {
    /// The caller whose session the request goes to.
    pub session: Caller,
    /// The request.
    pub request: R,
}

/// Rule 2: the session's identity.
pub fn key(caller: &Caller) -> String {
    match caller {
        Caller::Agent(agent) => format!("agent:{}:{}", agent.key.template, agent.key.index),
        Caller::Client { identity } => format!("client:{}:{}", identity.issuer, identity.subject),
    }
}

/// A request a session can carry: its own bytes, and back from them.
pub trait Carried: Sized {
    /// The request's own bytes; empty for a request that has none.
    fn params(&self) -> Result<Vec<u8>, serde_json::Error>;

    /// The request read back from its own bytes.
    fn from_params(bytes: &[u8]) -> Result<Self, serde_json::Error>;
}

/// The four whose bytes are their params as JSON.
macro_rules! carried {
    ($($request:ty),*) => {$(
        impl Carried for $request {
            fn params(&self) -> Result<Vec<u8>, serde_json::Error> {
                let mut bytes = Vec::new();
                self.encode(&mut Writer::new(&mut bytes))?;
                Ok(bytes)
            }

            fn from_params(bytes: &[u8]) -> Result<Self, serde_json::Error> {
                <$request>::decode(bytes)
            }
        }
    )*};
}

carried!(
    list_tools::request::Request,
    list_resources::request::Request,
    call_tool::request::Request,
    read_resource::request::Request
);

/// The notifications request, which has no bytes.
impl Carried for notifications::request::Request {
    fn params(&self) -> Result<Vec<u8>, serde_json::Error> {
        Ok(Vec::new())
    }

    fn from_params(_bytes: &[u8]) -> Result<Self, serde_json::Error> {
        Ok(notifications::request::Request)
    }
}

/// Rule 1, written.
impl<R: Carried> Encode for Sessioned<R> {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        let params = self.request.params()?;
        out.extend_from_slice(br#"{"session":"#);
        serde_json::to_writer(&mut *out, &self.session)?;
        if !params.is_empty() {
            out.extend_from_slice(br#","params":"#);
            out.extend_from_slice(&params);
        }
        out.extend_from_slice(b"}");
        Ok(())
    }
}

/// Rule 1, read: the object, then the request from its `params` —
/// `null` when absent, which the notifications request ignores and a
/// list reads as no params.
impl<R: Carried> Decode<'_> for Sessioned<R> {
    /// The ordinary JSON failure, of the object or of the request.
    type Error = serde_json::Error;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        #[derive(Deserialize)]
        struct Envelope {
            session: Caller,
            #[serde(default)]
            params: serde_json::Value,
        }
        let envelope: Envelope = serde_json::from_slice(bytes)?;
        let request = R::from_params(&serde_json::to_vec(&envelope.params)?)?;
        Ok(Sessioned { session: envelope.session, request })
    }
}
