use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One reply the calling client's `McpHandler` produced for a
/// [`super::super::server_request::Request`]. The typed
/// [`super::Payload`] variant pairs by name with the request side;
/// every method-specific result or error rides inside it. `mcp_kind`
/// echoes the request's so the API can sanity-check the routing
/// without trusting `id` alone.
///
/// Wire shape (envelope is `{id, mcp_kind, type, …variant fields…}`
/// after the `#[serde(flatten)]` on `payload`):
///
/// ```json
/// {
///   "id":"…",
///   "mcp_kind":{"type":"objective_ai"},
///   "type":"tools_list",
///   "kind":"ok",
///   "result":{…}
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_response.Response")]
pub struct Response {
    /// Matches the `id` of the
    /// [`super::super::server_request::Request`] this response is for.
    pub id: String,
    /// The typed response variant. The MCP-routed variants echo
    /// `mcp_kind` inside the variant itself (see [`super::Payload`]);
    /// non-MCP variants don't.
    #[serde(flatten)]
    pub payload: super::Payload,
}

impl Response {
    /// Which CLI-hosted MCP server this response came from. `Some` for
    /// the MCP-routed variants; `None` for non-MCP variants. Delegates
    /// to [`super::Payload::mcp_kind`].
    pub fn mcp_kind(&self) -> Option<super::super::McpKind> {
        self.payload.mcp_kind()
    }

    /// Serialize for the wire: JSON text for ordinary responses, the
    /// [`crate::binary_frame`] sandwich for a chunk-bearing successful
    /// `LaboratoryExportRead` (VARIANT-keyed — always binary, even
    /// with an empty payload).
    pub fn to_wire(
        &self,
    ) -> Result<crate::binary_frame::WireFrame, serde_json::Error> {
        let header = serde_json::to_string(self)?;
        Ok(match &self.payload {
            super::Payload::LaboratoryExportRead(super::JsonRpcResult::Ok {
                result,
            }) => crate::binary_frame::WireFrame::Binary(
                crate::binary_frame::encode(&header, &result.data),
            ),
            _ => crate::binary_frame::WireFrame::Text(header),
        })
    }

    /// Parse a BINARY wire frame. `None` for anything that isn't a
    /// well-formed sandwich around a chunk-bearing reply (receivers
    /// drop it — the forward-compat posture).
    pub fn from_binary(frame: &[u8]) -> Option<Self> {
        let (header, payload) = crate::binary_frame::decode(frame)?;
        let mut parsed: Self = serde_json::from_str(header).ok()?;
        match &mut parsed.payload {
            super::Payload::LaboratoryExportRead(super::JsonRpcResult::Ok {
                result,
            }) => {
                result.data = payload.to_vec();
                Some(parsed)
            }
            _ => None,
        }
    }
}
