use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Constructor-level options for a `Codex` client. Mirrors `CodexOptions`
/// in the Python SDK (`types.py`). Pydantic uses `populate_by_name=True`
/// with camelCase aliases, so the wire form may use either snake_case or
/// camelCase keys — both are accepted on deserialize.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CodexOptions {
    #[serde(
        rename = "codex_path_override",
        alias = "codexPathOverride",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub codex_path_override: Option<String>,

    #[serde(
        rename = "base_url",
        alias = "baseUrl",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub base_url: Option<String>,

    #[serde(
        rename = "api_key",
        alias = "apiKey",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub api_key: Option<String>,

    /// When provided, the SDK does not inherit from the parent process
    /// environment — only these variables are passed through.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<IndexMap<String, String>>,
}
