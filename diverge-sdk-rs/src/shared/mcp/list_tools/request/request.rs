//! Asking what tools there are.

use rmcp::model::{PaginatedRequestParams};
use serde_json::Error;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Where in the listing to start, if anywhere in particular.
///
/// [`None`] asks for the first page, and is also what a caller with
/// nothing to say sends. It is optional because
/// [`rmcp`](rmcp::model::ListToolsRequest) makes it optional: params
/// absent and a cursor absent are different things to a server that
/// reads them.
///
/// # It is a wrapper, and a thin one
///
/// [`rmcp`] already defines the shape and this crate has no business
/// redefining it — an MCP request has one form and it is MCP's. What
/// the wrapper adds is the two impls beneath it, so that a channel
/// request carrying one writes `request.encode(out)` like every other
/// payload in this crate rather than reaching for `serde_json` itself.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Request(
    /// The parameters, as [`rmcp`] defines them.
    pub Option<PaginatedRequestParams>,
);

/// Its JSON, and nothing in front of it. The tag that says which
/// request this is belongs to whichever frame carries it.
impl Encode for Request {
    /// The ordinary JSON failure.
    type Error = Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Error> {
        serde_json::to_writer(out, &self.0)
    }
}

impl Decode<'_> for Request {
    /// The ordinary JSON failure. There is nothing else here to get
    /// wrong — no tag to be unknown, and no empty case, since no bytes
    /// at all is a JSON document that ended too early and is reported
    /// as one.
    type Error = Error;

    fn decode(bytes: &[u8]) -> Result<Self, Error> {
        serde_json::from_slice(bytes).map(Request)
    }
}
