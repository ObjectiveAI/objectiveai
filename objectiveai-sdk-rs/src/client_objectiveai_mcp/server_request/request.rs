use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One reverse-attach request the API has shipped to the calling
/// client. The proxy's HTTP method (`POST` for the five JSON-RPC
/// methods, `DELETE` for session terminate) is implicit in the
/// [`super::Payload`] variant; the JSON-RPC `{jsonrpc, id, method,
/// params}` envelope is unwrapped into the typed variant payload.
///
/// Which CLI-hosted MCP server the request targets rides as
/// `mcp_kind` on the envelope
/// ([`super::super::McpKind::PluginLaboratory`] from the plugin's
/// typed marker; the two laboratory kinds from the laboratory
/// marker).
///
/// Wire shape (envelope is `{id, mcp_kind, headers?, type, …variant
/// fields…}` after the `#[serde(flatten)]` on `payload`):
///
/// ```json
/// {
///   "id":"…",
///   "mcp_kind":{"type":"plugin_laboratory","owner":"…","name":"…","version":"…"},
///   "headers":{"Mcp-Session-Id":"…"},
///   "type":"tools_list",
///   "cursor":"…"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.Request")]
pub struct Request {
    /// Server-minted correlation id. Echoed by the matching
    /// [`super::super::server_response::Response`].
    pub id: String,
    /// Verbatim copy of the headers the proxy sent on its HTTP
    /// request to the API. The CLI conduit reads several custom
    /// `X-OBJECTIVEAI-*` routing headers + `Mcp-Session-Id` off this
    /// map; protocol-level headers (Host, Content-Length, …) the API
    /// already stripped on its way in.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub headers: IndexMap<String, String>,
    /// The typed request variant. The MCP-routed variants carry
    /// `mcp_kind` inside the variant itself (see [`super::Payload`]);
    /// non-MCP variants don't.
    #[serde(flatten)]
    pub payload: super::Payload,
}

impl Request {
    /// Which CLI-hosted MCP server this request targets. `Some` for
    /// the MCP-routed variants (`Initialize` / `ToolsList` /
    /// `ToolsCall` / `ResourcesList` / `ResourcesRead` /
    /// `SessionTerminate`); `None` for non-MCP variants
    /// (`ReadMessageQueue` / `ClearMessageQueue`) which hit the CLI's
    /// own local state. Delegates to [`super::Payload::mcp_kind`].
    pub fn mcp_kind(&self) -> Option<super::super::McpKind> {
        self.payload.mcp_kind()
    }

    /// Serialize for the wire: JSON text for ordinary requests, the
    /// [`crate::binary_frame`] sandwich for the chunk-bearing
    /// `LaboratoryImportWrite` (VARIANT-keyed — always binary, even
    /// with an empty payload).
    pub fn to_wire(
        &self,
    ) -> Result<crate::binary_frame::WireFrame, serde_json::Error> {
        let header = serde_json::to_string(self)?;
        Ok(match &self.payload {
            super::Payload::LaboratoryImportWrite(req) => {
                crate::binary_frame::WireFrame::Binary(
                    crate::binary_frame::encode(&header, &req.data),
                )
            }
            _ => crate::binary_frame::WireFrame::Text(header),
        })
    }

    /// Parse a BINARY wire frame. `None` for anything that isn't a
    /// well-formed sandwich around a chunk-bearing request (receivers
    /// drop it — the forward-compat posture).
    pub fn from_binary(frame: &[u8]) -> Option<Self> {
        let (header, payload) = crate::binary_frame::decode(frame)?;
        let mut parsed: Self = serde_json::from_str(header).ok()?;
        match &mut parsed.payload {
            super::Payload::LaboratoryImportWrite(req) => {
                req.data = payload.to_vec();
                Some(parsed)
            }
            _ => None,
        }
    }
}
