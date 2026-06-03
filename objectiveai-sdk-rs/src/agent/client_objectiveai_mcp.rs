//! Client-side ObjectiveAI MCP surface declared by an agent.
//!
//! Sits alongside [`super::McpServers`] on every upstream
//! `AgentBase`. Where `mcp_servers` lists full-URL MCP servers the
//! agent will dial out to, this struct declares the
//! ObjectiveAI-managed bits (the built-in `objectiveai-mcp` plus
//! specific plugins / tools by `owner`+`name`+`version`) the agent
//! expects the *calling client* to expose locally back to the API.
//!
//! Content-addressed: the field flows into each upstream's `id()`
//! hash, so swapping in a different plugin reference produces a
//! different agent id.

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A single `owner` / `name` / `version` reference identifying one
/// tool inside [`ClientObjectiveaiMcp::tools`]. Plugin references
/// use the larger [`ClientObjectiveaiMcpPluginEntry`] — they carry
/// extra `executable` / `mcp_servers` fields tools don't have.
#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.ClientObjectiveaiMcpEntry")]
pub struct ClientObjectiveaiMcpEntry {
    pub owner: String,
    pub name: String,
    pub version: String,
}

impl ClientObjectiveaiMcpEntry {
    /// `owner`, `name`, and `version` must all be non-empty.
    pub fn validate(&self) -> Result<(), String> {
        if self.owner.is_empty() {
            return Err("`owner` cannot be empty".into());
        }
        if self.name.is_empty() {
            return Err("`name` cannot be empty".into());
        }
        if self.version.is_empty() {
            return Err("`version` cannot be empty".into());
        }
        Ok(())
    }

    /// LLM-visible tool name. See [`materialize_tool_name`].
    pub fn tool_name(&self) -> String {
        materialize_tool_name(&self.owner, &self.name, &self.version)
    }
}

/// Plugin reference inside [`ClientObjectiveaiMcp::plugins`].
///
/// - `owner` / `name` / `version` identify the plugin (same shape as
///   [`ClientObjectiveaiMcpEntry`]).
/// - `executable` controls whether this plugin contributes a tool
///   to the agent's surface (`true`, default) or is loaded purely
///   for its declared MCP servers (`false`). The API's
///   `X-OBJECTIVEAI-TOOLS-ALLOWED` construction honors this: only
///   `executable = true` plugin entries contribute their `name` to
///   the allow-list.
/// - `mcp_servers` selects which of the plugin's manifest-declared
///   `filesystem::plugins::Manifest::mcp_servers` entries should be
///   exposed to the agent, by `name`. `None` ⇒ none of them.
#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.ClientObjectiveaiMcpPluginEntry")]
pub struct ClientObjectiveaiMcpPluginEntry {
    pub owner: String,
    pub name: String,
    pub version: String,
    /// `true`: spawn the plugin binary and surface its tools the
    /// usual way. `false`: don't run the plugin; only consume its
    /// declared MCP servers (`mcp_servers` below). Defaults to
    /// `true` so existing declarations keep their current behavior.
    #[serde(default = "default_true")]
    pub executable: bool,
    /// Subset of the plugin's manifest `mcp_servers` to expose, each
    /// referenced by `name` plus an optional `arguments` map. `None`
    /// ⇒ none. Names that aren't present in the plugin's manifest are
    /// rejected when the API asks the CLI to begin them; declarations
    /// themselves don't validate the referent (the plugin may not be
    /// installed at declaration time).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub mcp_servers: Option<Vec<ClientObjectiveaiMcpPluginMcpServer>>,
}

/// One `mcp_servers` entry on a
/// [`ClientObjectiveaiMcpPluginEntry`]: the manifest-declared `name`
/// the agent wants exposed, plus optional `arguments` the CLI feeds
/// to the plugin alongside the name when bringing the server up. The
/// arguments map is sorted by key in [`prepare`] so two equivalent
/// declarations (same key/value pairs in any order) hash to the same
/// canonical form.
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema, arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.ClientObjectiveaiMcpPluginMcpServer")]
pub struct ClientObjectiveaiMcpPluginMcpServer {
    /// Author-chosen identifier — must match a `name` in the plugin
    /// manifest's `mcp_servers` list. The CLI feeds this as the
    /// first positional arg when starting the plugin.
    pub name: String,
    /// Optional key→value arguments forwarded to the plugin alongside
    /// `name`. `Some(value)` ⇒ `--key value` on the spawned plugin's
    /// argv; `None` ⇒ a bare `--key` flag with no following token. The
    /// plugin author decides how to interpret them. [`prepare`]
    /// normalizes (`Some("") → None`), sorts the map by key, and
    /// collapses an empty map to `None` so two equivalent declarations
    /// canonicalize to byte-identical JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_indexmap_string_option_string)]
    pub arguments: Option<IndexMap<String, Option<String>>>,
}

impl PartialOrd for ClientObjectiveaiMcpPluginMcpServer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ClientObjectiveaiMcpPluginMcpServer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Compare by name first. `IndexMap` doesn't derive `Ord`, so
        // we walk the entries in iteration order — after `prepare`'s
        // `sort_keys` pass that order is deterministic. `None`
        // arguments sorts before `Some(...)` via the standard
        // `Option<T>::cmp` ordering (so a bare `--flag` sorts before
        // `--flag value`).
        let by_name = self.name.cmp(&other.name);
        if by_name.is_ne() {
            return by_name;
        }
        let a: Option<Vec<(&String, &Option<String>)>> =
            self.arguments.as_ref().map(|m| m.iter().collect());
        let b: Option<Vec<(&String, &Option<String>)>> =
            other.arguments.as_ref().map(|m| m.iter().collect());
        a.cmp(&b)
    }
}

impl ClientObjectiveaiMcpPluginMcpServer {
    /// `name` must be non-empty, and every `arguments` key (if
    /// present) must be non-empty (values may be empty).
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("`name` cannot be empty".into());
        }
        if let Some(args) = self.arguments.as_ref() {
            for (k, _) in args {
                if k.is_empty() {
                    return Err("`arguments` key cannot be empty".into());
                }
            }
        }
        Ok(())
    }
}

fn default_true() -> bool {
    true
}

impl ClientObjectiveaiMcpPluginEntry {
    /// `owner`, `name`, and `version` must all be non-empty; each
    /// `mcp_servers[i]` must validate (see
    /// [`ClientObjectiveaiMcpPluginMcpServer::validate`]); and within
    /// one plugin entry, `mcp_servers` must contain no duplicate
    /// `name` values (matches the uniqueness rule the plugin manifest
    /// itself enforces at
    /// `crate::filesystem::plugins::Manifest::validate`).
    pub fn validate(&self) -> Result<(), String> {
        if self.owner.is_empty() {
            return Err("`owner` cannot be empty".into());
        }
        if self.name.is_empty() {
            return Err("`name` cannot be empty".into());
        }
        if self.version.is_empty() {
            return Err("`version` cannot be empty".into());
        }
        if let Some(servers) = self.mcp_servers.as_ref() {
            for entry in servers {
                entry.validate()?;
            }
            for (i, a) in servers.iter().enumerate() {
                for b in &servers[i + 1..] {
                    if a.name == b.name {
                        return Err(format!(
                            "`mcp_servers` contains duplicate name: \"{}\"",
                            a.name
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// LLM-visible tool name. See [`materialize_tool_name`].
    pub fn tool_name(&self) -> String {
        materialize_tool_name(&self.owner, &self.name, &self.version)
    }
}

/// Materialize the LLM-visible tool name for an `owner` / `name` /
/// `version` triple: `{owner}-{name}-{version}` with every `.`
/// substituted to `-`. The substitution keeps the result
/// Anthropic-tool-name-regex safe (`^[a-zA-Z0-9_-]{1,128}$`) even
/// when the version field carries semver dots (`1.2.3` -> `1-2-3`).
///
/// Single source of truth shared by [`ClientObjectiveaiMcpEntry::tool_name`],
/// [`crate::filesystem::plugins::Manifest::tool_name`], and
/// [`crate::filesystem::tools::Manifest::tool_name`].
pub fn materialize_tool_name(owner: &str, name: &str, version: &str) -> String {
    format!("{owner}-{name}-{version}").replace('.', "-")
}

/// Client-side MCP surface the agent expects:
///
/// - `objectiveai`: whether the calling client exposes the built-in
///   `objectiveai-mcp`. `None` means unspecified; `Some(true)` /
///   `Some(false)` explicitly opt in / out.
/// - `plugins`: specific plugins (by `owner` / `name` / `version`)
///   plus per-plugin `executable` + `mcp_servers`.
/// - `tools`: specific tools (by `owner` / `name` / `version`).
#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    arbitrary::Arbitrary,
    Default,
)]
#[schemars(rename = "agent.ClientObjectiveaiMcp")]
pub struct ClientObjectiveaiMcp {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub objectiveai: Option<bool>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub plugins: Vec<ClientObjectiveaiMcpPluginEntry>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub tools: Vec<ClientObjectiveaiMcpEntry>,
}

/// Validates the configuration. Each entry's fields must be
/// non-empty, and the `plugins` / `tools` lists each contain no
/// `(owner, name, version)` duplicates. Free-function counterpart to
/// [`super::mcp_servers::validate`].
pub fn validate(this: &ClientObjectiveaiMcp) -> Result<(), String> {
    for entry in &this.plugins {
        entry.validate()?;
    }
    for entry in &this.tools {
        entry.validate()?;
    }
    for (i, a) in this.plugins.iter().enumerate() {
        for b in &this.plugins[i + 1..] {
            if a.owner == b.owner && a.name == b.name && a.version == b.version
            {
                return Err(format!(
                    "`client_objectiveai_mcp.plugins` contains duplicate entry: \"{}/{}@{}\"",
                    a.owner, a.name, a.version,
                ));
            }
        }
    }
    for (i, a) in this.tools.iter().enumerate() {
        for b in &this.tools[i + 1..] {
            if a == b {
                return Err(format!(
                    "`client_objectiveai_mcp.tools` contains duplicate entry: \"{}/{}@{}\"",
                    a.owner, a.name, a.version,
                ));
            }
        }
    }
    Ok(())
}

/// Sorts plugins + tools for deterministic ordering. Per-plugin
/// `mcp_servers` get their inner `arguments` IndexMap key-sorted
/// in place, then the `mcp_servers` Vec is sorted via the
/// [`ClientObjectiveaiMcpPluginMcpServer`] `Ord` impl. Collapses an
/// all-empty struct to `None` so the enclosing `Option` can drop the
/// empty container entirely (same convention as
/// [`super::mcp_servers::prepare`]).
pub fn prepare(mut this: ClientObjectiveaiMcp) -> Option<ClientObjectiveaiMcp> {
    for plugin in &mut this.plugins {
        if let Some(servers) = plugin.mcp_servers.as_mut() {
            for entry in servers.iter_mut() {
                // Normalize each value (`Some("") → None` so an
                // explicit empty string canonicalizes the same way
                // as a missing value — i.e., a bare `--flag`), sort
                // by key, then collapse an empty map to `None` so
                // the canonical form omits the field entirely.
                let drop_empty = match entry.arguments.as_mut() {
                    Some(args) => {
                        for (_, v) in args.iter_mut() {
                            if let Some(s) = v.as_deref() {
                                if s.is_empty() {
                                    *v = None;
                                }
                            }
                        }
                        args.sort_keys();
                        args.is_empty()
                    }
                    None => false,
                };
                if drop_empty {
                    entry.arguments = None;
                }
            }
            servers.sort();
        }
    }
    this.plugins.sort();
    this.tools.sort();
    if this.objectiveai.is_none() && this.plugins.is_empty() && this.tools.is_empty() {
        None
    } else {
        Some(this)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, args: &[(&str, Option<&str>)]) -> ClientObjectiveaiMcpPluginMcpServer {
        let arguments = if args.is_empty() {
            None
        } else {
            let mut m = IndexMap::new();
            for (k, v) in args {
                m.insert(k.to_string(), v.map(|s| s.to_string()));
            }
            Some(m)
        };
        ClientObjectiveaiMcpPluginMcpServer {
            name: name.to_string(),
            arguments,
        }
    }

    fn plugin(name: &str, servers: Vec<ClientObjectiveaiMcpPluginMcpServer>) -> ClientObjectiveaiMcpPluginEntry {
        ClientObjectiveaiMcpPluginEntry {
            owner: "o".into(),
            name: name.into(),
            version: "v".into(),
            executable: true,
            mcp_servers: Some(servers),
        }
    }

    fn shell(plugins: Vec<ClientObjectiveaiMcpPluginEntry>) -> ClientObjectiveaiMcp {
        ClientObjectiveaiMcp {
            objectiveai: None,
            plugins,
            tools: vec![],
        }
    }

    #[test]
    fn prepare_sorts_arguments_by_key_so_order_does_not_matter() {
        let a = shell(vec![plugin(
            "p",
            vec![entry("s", &[("b", Some("1")), ("a", Some("2"))])],
        )]);
        let b = shell(vec![plugin(
            "p",
            vec![entry("s", &[("a", Some("2")), ("b", Some("1"))])],
        )]);
        let ap = prepare(a).expect("non-empty after prepare");
        let bp = prepare(b).expect("non-empty after prepare");
        assert_eq!(
            serde_json::to_string(&ap).unwrap(),
            serde_json::to_string(&bp).unwrap(),
            "two declarations with identical key/value pairs in different insertion order must canonicalize to byte-identical JSON",
        );
    }

    #[test]
    fn prepare_sorts_mcp_servers_vec_by_name_then_arguments() {
        let a = shell(vec![plugin(
            "p",
            vec![
                entry("z", &[("a", Some("1"))]),
                entry("a", &[("k", Some("v"))]),
            ],
        )]);
        let ap = prepare(a).expect("non-empty after prepare");
        let servers = ap.plugins[0].mcp_servers.as_ref().unwrap();
        assert_eq!(servers[0].name, "a");
        assert_eq!(servers[1].name, "z");
    }

    #[test]
    fn validate_rejects_duplicate_mcp_server_names_within_plugin() {
        let bad = shell(vec![plugin(
            "p",
            vec![entry("dup", &[]), entry("dup", &[("k", Some("v"))])],
        )]);
        let err = validate(&bad).expect_err("duplicate names must be rejected");
        assert!(err.contains("duplicate name"), "unexpected error: {err}");
    }

    #[test]
    fn validate_rejects_empty_argument_key() {
        let bad = shell(vec![plugin("p", vec![entry("s", &[("", Some("v"))])])]);
        let err = validate(&bad).expect_err("empty argument keys must be rejected");
        assert!(err.contains("`arguments` key"), "unexpected error: {err}");
    }

    #[test]
    fn empty_arguments_round_trip_omits_field() {
        let s = entry("name", &[]);
        let json = serde_json::to_string(&s).unwrap();
        assert!(
            !json.contains("arguments"),
            "absent arguments must be skipped on serialize: {json}"
        );
        let back: ClientObjectiveaiMcpPluginMcpServer = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn populated_arguments_round_trip() {
        // Mix of valued and flag-only args.
        let s = entry("name", &[("a", Some("1")), ("debug", None), ("b", Some("2"))]);
        let json = serde_json::to_string(&s).unwrap();
        let back: ClientObjectiveaiMcpPluginMcpServer = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn prepare_normalizes_empty_string_value_to_none() {
        // Caller wrote `--debug ""` — prepare must canonicalize that
        // to a bare `--debug` flag (None) so identical declarations
        // hash byte-identically regardless of whether the author
        // wrote `None` or `Some("")`.
        let with_empty = shell(vec![plugin("p", vec![entry("s", &[("debug", Some(""))])])]);
        let prepared = prepare(with_empty).expect("non-empty after prepare");
        let args = prepared.plugins[0].mcp_servers.as_ref().unwrap()[0]
            .arguments
            .as_ref()
            .unwrap();
        assert_eq!(
            args.get("debug").unwrap(),
            &None,
            "Some(\"\") must canonicalize to None"
        );

        // And the canonical JSON for `Some("")` must equal the one
        // for `None`.
        let with_none = shell(vec![plugin("p", vec![entry("s", &[("debug", None)])])]);
        let prepared_none = prepare(with_none).expect("non-empty after prepare");
        assert_eq!(
            serde_json::to_string(&prepared).unwrap(),
            serde_json::to_string(&prepared_none).unwrap(),
        );
    }

    #[test]
    fn prepare_collapses_empty_arguments_to_none() {
        // Caller explicitly supplied `Some(empty map)` — prepare
        // must canonicalize that to `None` so it serializes
        // identically to the absent case.
        let with_empty = ClientObjectiveaiMcp {
            objectiveai: None,
            plugins: vec![ClientObjectiveaiMcpPluginEntry {
                owner: "o".into(),
                name: "p".into(),
                version: "v".into(),
                executable: true,
                mcp_servers: Some(vec![ClientObjectiveaiMcpPluginMcpServer {
                    name: "s".into(),
                    arguments: Some(IndexMap::new()),
                }]),
            }],
            tools: vec![],
        };
        let prepared = prepare(with_empty).expect("non-empty after prepare");
        let arg = &prepared.plugins[0].mcp_servers.as_ref().unwrap()[0].arguments;
        assert!(arg.is_none(), "empty arguments map must canonicalize to None");
    }
}
