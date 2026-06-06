/// Success-only response shape. Wire form is the bare string `"Ok"` —
/// a single-variant enum gives us a typed sentinel that serializes and
/// deserializes through serde as the static string. Used as `Response`
/// on every cli leaf whose only success signal is "it worked."
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.Ok")]
pub enum Ok {
    Ok,
}

#[cfg(feature = "mcp")]
impl super::CommandResponse for Ok {
    fn into_mcp(self) -> super::McpResponseItem {
        super::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}
