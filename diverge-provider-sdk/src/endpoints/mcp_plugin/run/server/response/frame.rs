//! What a server's response frame carries for an MCP plugin.

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::error::Error;

/// The plugin did not come up.
///
/// The only thing a run's scope ever says, and the whole payload — no
/// tag, because there is nothing to discriminate.
///
/// | the scope | means |
/// |-----------|-------|
/// | says nothing, and stays open | the plugin is running |
/// | one of these, then a finish | it never came up, and here is what the provider knows |
/// | a finish, with none of these | the run is over |
///
/// # Silence is the good case
///
/// Which is unusual here and worth stating plainly: a plugin that
/// works produces no response frame at all. The scope opens, the image
/// is pulled on channels the provider opens, the container starts, and
/// nothing is said about any of it. What a caller does next is call the
/// plugin.
///
/// # Why there is no readiness signal
///
/// There was one, and it was removed, because it could not mean what
/// it appeared to. A provider knows when a CONTAINER has started, and
/// that is not the same fact as the MCP server inside it having bound
/// [`port`](crate::endpoints::mcp_plugin::run::client::request::Frame::port).
/// A signal sent at the first would have been read as the second.
///
/// The honest test is a call. This endpoint already relies on that
/// elsewhere — a wrong `port` is documented as surfacing "as an
/// exchange that finishes without an answer, rather than when the
/// plugin started" — so a caller that wants to know whether the plugin
/// is up asks it something, and a readiness frame would have been a
/// second, weaker answer to a question already answered better.
///
/// It also cost a round of doubt that a caller had no way to resolve:
/// nothing said what to do about a plugin that reported ready and then
/// did not answer.
///
/// # Why there is no id
///
/// A [`laboratory run`](crate::endpoints::laboratories::run) answers
/// with one, because a laboratory is a place others join: an id is
/// what a [`connect`](crate::endpoints::laboratories::connect) names
/// and what a
/// [`transfer`](crate::shared::container::transfer) sends a file to.
///
/// A plugin is none of those. It is created for one caller, answers
/// that caller's tool calls, and is torn down after — so the scope is
/// the whole of the handle, and an id would name a thing nobody can
/// address. Minting one anyway would be inventing a way to reach a
/// plugin container that this specification deliberately does not
/// offer.
///
/// # Why there is no filetree either
///
/// Because nothing reads it. A laboratory reports its filesystem
/// because an agent works in it and wants to see what it is working
/// on. A plugin serves tools; its filesystem is its author's business,
/// it takes no [`mounts`], and a caller with no way to read or write
/// inside it has nothing to do with a tree of it.
///
/// # A struct, and no tag
///
/// This was an enum — a readiness signal beside the failure — and its
/// tag byte told the two apart. With one variant left there is nothing
/// to tell apart, so the byte went with the variant that justified it.
///
/// It is not held open against a second kind of answer arriving later.
/// A tag spent on a choice nobody is making is a byte on every frame
/// and a case in every reader, paid now for something that may never
/// happen — which is the same trade
/// [`write_path`](crate::shared::container::write_path::response::Frame)
/// looked at and declined.
///
/// [`mounts`]: crate::endpoints::laboratories::run::client::request::Frame::mounts
#[derive(Debug, Clone, PartialEq)]
pub struct Frame(
    /// What went wrong.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    pub Error,
);

/// The error's JSON, and nothing in front of it.
impl Encode for Frame {
    /// The ordinary JSON failure, which is the error's own.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this struct's field is a
    // type called `Error`, so the associated type is ambiguous by that
    // name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        self.0.encode(out)
    }
}

impl Decode<'_> for Frame {
    /// The ordinary JSON failure, which is the error's own. There is
    /// nothing else here to get wrong — no tag to be unknown, and no
    /// empty case, since no bytes at all is a JSON document that ended
    /// too early and is reported as one.
    type Error = serde_json::Error;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        Error::decode(bytes).map(Frame)
    }
}
