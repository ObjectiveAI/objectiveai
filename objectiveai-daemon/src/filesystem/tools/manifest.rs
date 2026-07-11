use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Per-OS exec command. Reused from the SDK so the exec shape stays
/// identical across the on-disk manifest and the `tools get` wire
/// response.
pub use objectiveai_sdk::cli::command::tools::get::Exec;

/// Per-CPU-architecture cli bundle filenames for a single OS. Each entry
/// is the GitHub-release `.zip` asset filename for that arch (or absent
/// when not built for it). Arch keys follow the repo-wide convention
/// (`x86_64` / `aarch64`) — the same names the release workflow uses for
/// its assets, which is exactly what these values reference.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "filesystem.tools.CliZipArch")]
pub struct CliZipArch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub x86_64: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub aarch64: Option<String>,
}

impl CliZipArch {
    /// True when no arch entry is present — used by [`CliZip`]'s
    /// `skip_serializing_if` so an OS with no bundles is omitted (and a
    /// fully-empty `cli_zip` still serializes as `{}`).
    pub fn is_empty(&self) -> bool {
        self.x86_64.is_none() && self.aarch64.is_none()
    }
}

/// Per-OS cli bundle: the GitHub-release `.zip` asset filenames for each
/// platform, split per CPU architecture (`x86_64` / `aarch64`). At
/// install time the current platform's entry — matched on OS *and* arch —
/// is downloaded and extracted into the `cli/` working directory. Local
/// to the CLI's filesystem layer (the on-disk shape); shared by both the
/// tool and plugin manifests. The `cli_zip` field itself is required;
/// each per-OS and per-arch entry is optional — absent means nothing to
/// fetch for that platform (e.g. a hand-placed source-run tool, or a
/// viewer-only plugin).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "filesystem.tools.CliZip")]
pub struct CliZip {
    #[serde(default, skip_serializing_if = "CliZipArch::is_empty")]
    pub windows: CliZipArch,
    #[serde(default, skip_serializing_if = "CliZipArch::is_empty")]
    pub linux: CliZipArch,
    #[serde(default, skip_serializing_if = "CliZipArch::is_empty")]
    pub macos: CliZipArch,
}

/// Declarative metadata a local tool ships with — the on-disk
/// `objectiveai.json` at
/// `<base_dir>/tools/<owner>/<name>/<version>/objectiveai.json`. The
/// CLI reads and writes this shape verbatim; the `tools get` wire
/// response is a lean projection of it (it drops `cli_zip`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "filesystem.tools.Manifest")]
pub struct Manifest {
    /// GitHub-style owner (user or org) of the tool's source repo.
    pub owner: String,
    /// The tool's identifier (matches the `<name>` directory segment).
    pub name: String,
    /// Version string. Free-form; semver recommended, not enforced.
    pub version: String,
    /// One-line description of what the tool does. Surfaced to agents
    /// that consume the tool.
    pub description: String,
    /// Per-OS exec command. The current platform's vector is the
    /// program plus its leading arguments; the caller's `--args` are
    /// appended and the whole thing runs with CWD = this version
    /// folder's `cli/` subdir.
    pub exec: Exec,
    /// Per-OS cli bundle zip filenames (required field; each per-OS
    /// entry optional). Hand-placed tools leave every entry absent.
    pub cli_zip: CliZip,
}

impl Manifest {
    /// Validate fields that serde alone can't enforce. Tools have no
    /// cross-field invariants (unlike plugins' viewer rules), so this is
    /// currently a no-op — it exists so the shared install/discovery
    /// paths can call `validate()` uniformly across manifest kinds.
    pub fn validate(&self) -> Result<(), &'static str> {
        Ok(())
    }
}

// Projection to the SDK's lean `tools get` wire response — drops the
// on-disk-only `cli_zip`. `exec` is the same SDK `Exec` type, so it
// moves across unchanged.
impl From<Manifest> for objectiveai_sdk::cli::command::tools::get::ResponseManifest {
    fn from(m: Manifest) -> Self {
        Self {
            owner: m.owner,
            name: m.name,
            version: m.version,
            description: m.description,
            exec: m.exec,
        }
    }
}
