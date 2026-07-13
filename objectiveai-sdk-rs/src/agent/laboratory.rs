//! Laboratories embedded in agent definitions.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The reserved id namespace for agent laboratories. A laboratory
/// whose id starts with this prefix is DERIVED from an agent (see
/// [`laboratories::derived_id`]) — never user-created, never
/// attachable or detachable.
pub const AGENT_LABORATORY_ID_PREFIX: &str = "oai-agent-";

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
    /// valid, and no duplicate entries may exist (duplicates would
    /// derive the same laboratory id).
    pub fn validate(this: &super::Laboratories) -> Result<(), String> {
        for laboratory in this {
            laboratory
                .image
                .validate()
                .map_err(|message| format!("`laboratories` image: {message}"))?;
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
}
