//! Laboratories embedded in agent definitions.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The reserved id namespace for agent laboratories. A laboratory
/// whose id starts with this prefix is DERIVED from an agent (see
/// [`laboratories::derived_id`]) — never user-created, never
/// attachable or detachable.
pub const AGENT_LABORATORY_ID_PREFIX: &str = "oai-agent-";

/// RESERVED environment variable names in laboratory containers: the
/// full agent-argument env surface of the CLI config — the six
/// agent-identity values (stamped from the create request's transient
/// headers) plus the plugin trio (stamped from the CANONICAL plugin
/// coordinates on plugin containers; never wire-derived — the host is
/// the authority). The laboratory HOST stamps these on every
/// ephemeral (agent and plugin) container at create, so a
/// user-declared value could only be overridden or an identity spoof
/// — [`laboratories::validate`] rejects them from agent-laboratory
/// `env` outright.
pub const RESERVED_LABORATORY_ENV: &[&str] = &[
    "OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY",
    "OBJECTIVEAI_AGENT_ID",
    "OBJECTIVEAI_AGENT_FULL_ID",
    "OBJECTIVEAI_AGENT_REMOTE",
    "OBJECTIVEAI_RESPONSE_ID",
    "OBJECTIVEAI_RESPONSE_IDS",
    "OBJECTIVEAI_PLUGIN_OWNER",
    "OBJECTIVEAI_PLUGIN_REPOSITORY",
    "OBJECTIVEAI_PLUGIN_VERSION",
];

/// A laboratory provisioned for an agent: the container spec the CLI
/// conduit materializes on demand at MCP-initialize. No mounts —
/// agent laboratories don't support them.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.Laboratory")]
pub struct Laboratory {
    /// The base image — an inline Containerfile XOR a split registry
    /// reference.
    pub image: crate::laboratories::LaboratoryImage,
    /// `[key, value]` environment pairs. Sorted by prepare; empty
    /// collapses to `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub env: Option<Vec<[String; 2]>>,
    /// Working directory new agents start in. `"/"` (the create-time
    /// default) collapses to `None` in prepare.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub cwd: Option<String>,
}

/// A list of agent laboratories.
pub type Laboratories = Vec<Laboratory>;

pub mod laboratories {
    //! Functions for working with [`Laboratories`](super::Laboratories).

    use twox_hash::XxHash3_128;

    /// Normalizes for the content-addressed agent id: per entry, an
    /// empty `env` collapses to `None` (else sorts), a `cwd` of `"/"`
    /// (the create-time default) collapses to `None`; the entries then
    /// sort as a whole; an empty list collapses to `None` — identical
    /// specs always serialize to identical bytes.
    pub fn prepare(
        mut this: super::Laboratories,
    ) -> Option<super::Laboratories> {
        if this.is_empty() {
            return None;
        }
        for laboratory in &mut this {
            laboratory.env = match laboratory.env.take() {
                Some(mut env) if !env.is_empty() => {
                    env.sort();
                    Some(env)
                }
                _ => None,
            };
            if laboratory.cwd.as_deref() == Some("/") {
                laboratory.cwd = None;
            }
        }
        this.sort();
        Some(this)
    }

    /// Validates all laboratories in the list: each image must be
    /// valid, no `env` key may use a
    /// [reserved identity name](super::RESERVED_LABORATORY_ENV) (the
    /// laboratory host stamps those), and no duplicate entries may
    /// exist (duplicates would derive the same laboratory id).
    pub fn validate(this: &super::Laboratories) -> Result<(), String> {
        for laboratory in this {
            laboratory
                .image
                .validate()
                .map_err(|message| format!("`laboratories` image: {message}"))?;
            for [key, _] in laboratory.env.as_deref().unwrap_or(&[]) {
                if super::RESERVED_LABORATORY_ENV.contains(&key.as_str()) {
                    return Err(format!(
                        "`laboratories` env {key:?} is reserved — the laboratory host stamps the agent-identity environment authoritatively",
                    ));
                }
            }
        }
        for (i, a) in this.iter().enumerate() {
            for b in &this[i + 1..] {
                if a == b {
                    return Err(
                        "`laboratories` contains duplicate entries".to_string()
                    );
                }
            }
        }
        Ok(())
    }

    /// The laboratory's DERIVED id: the reserved
    /// [`AGENT_LABORATORY_ID_PREFIX`](super::AGENT_LABORATORY_ID_PREFIX)
    /// plus a 22-character base62 XxHash3_128 over the agent's full id,
    /// a `0x00` separator, and the prepared spec's JSON — the same spec
    /// on the same agent always lands on the same laboratory.
    pub fn derived_id(
        agent_full_id: &str,
        laboratory: &super::Laboratory,
    ) -> String {
        let mut hasher = XxHash3_128::with_seed(0);
        hasher.write(agent_full_id.as_bytes());
        hasher.write(&[0u8]);
        hasher.write(serde_json::to_string(laboratory).unwrap().as_bytes());
        format!(
            "{}{:0>22}",
            super::AGENT_LABORATORY_ID_PREFIX,
            base62::encode(hasher.finish_128())
        )
    }

    /// The EPHEMERAL laboratory id for ONE agent-completion response:
    /// the content-addressed [`derived_id`] (which keys the cached
    /// agent image) plus the response id (which makes the container
    /// unique to its completion). Ephemeral laboratories live exactly
    /// as long as their single MCP connection.
    pub fn ephemeral_id(
        agent_full_id: &str,
        laboratory: &super::Laboratory,
        response_id: &str,
    ) -> String {
        format!("{}-{response_id}", derived_id(agent_full_id, laboratory))
    }
}
