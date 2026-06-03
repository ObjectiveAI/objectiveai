use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Declarative metadata a local tool ships with. The wire shape is
/// JSON: `<base_dir>/tools/<name>.json`. Companion executable lives
/// alongside in `<base_dir>/tools/<exec>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "filesystem.tools.Manifest")]
pub struct Manifest {
    /// One-line description of what the tool does. Surfaced to
    /// agents that consume the tool.
    pub description: String,

    /// Version string. Free-form; the host displays whatever's here
    /// (semver convention recommended but not enforced).
    pub version: String,

    /// GitHub-style owner (user or org) of the tool's source repo.
    /// Free-form; tools have no installer, so this is purely
    /// author-supplied metadata — nothing overrides it the way the
    /// plugin installer overrides the plugin manifest's owner.
    pub owner: String,

    /// Filename of the executable to invoke, resolved relative to
    /// `<base_dir>/tools/`. The author is responsible for including
    /// any platform-specific extension (`.exe`, `.sh`, `.bat`, …) —
    /// the SDK doesn't synthesise one.
    pub exec: String,
}

impl Manifest {
    /// LLM-visible tool name. See
    /// [`objectiveai_sdk::agent::materialize_tool_name`].
    pub fn tool_name(&self, name: &str) -> String {
        objectiveai_sdk::agent::materialize_tool_name(&self.owner, name, &self.version)
    }
}

/// A [`Manifest`] enriched with the tool's identifying `name` and the
/// `source` it was loaded from. Same shape and ordering convention
/// as `filesystem::plugins::ManifestWithNameAndSource`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "filesystem.tools.ManifestWithNameAndSource")]
pub struct ManifestWithNameAndSource {
    /// The tool's identifier — the filename it lives under in the
    /// tools directory (e.g. `hello` for `~/.objectiveai/tools/hello.json`).
    pub name: String,
    #[serde(flatten)]
    pub manifest: Manifest,
    /// Where this manifest came from — typically an absolute
    /// filesystem path. Free-form string; the host just displays it.
    pub source: String,
}

impl ManifestWithNameAndSource {
    /// LLM-visible tool name. See [`Manifest::tool_name`] — this
    /// helper supplies the `name` field automatically.
    pub fn tool_name(&self) -> String {
        self.manifest.tool_name(&self.name)
    }
}

// Typed conversion to the SDK's bare-naked wire shape. Lets
// `command::tools::{get, list}` leaves yield SDK `ResponseManifest`
// items without round-tripping through `serde_json::Value`.
impl From<ManifestWithNameAndSource>
    for objectiveai_sdk::cli::command::tools::get::ResponseManifest
{
    fn from(m: ManifestWithNameAndSource) -> Self {
        Self {
            name: m.name,
            description: m.manifest.description,
            version: m.manifest.version,
            owner: m.manifest.owner,
            exec: m.manifest.exec,
            source: m.source,
        }
    }
}
