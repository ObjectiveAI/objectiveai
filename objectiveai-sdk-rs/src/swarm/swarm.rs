//! Core Swarm types and validation logic.

use crate::agent;
use crate::weights::{Weights, WeightsEntry};
use indexmap::IndexMap;
use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use twox_hash::XxHash3_128;

// ── Pre-validation types (no computed ID) ──────────────────────────

/// An inline swarm base definition (without computed ID or metadata).
///
/// Contains a list of agent configurations that will be validated, deduplicated,
/// and sorted when converting to an [`InlineSwarm`].
#[derive(
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "swarm.InlineSwarmBase")]
pub struct InlineSwarmBase {
    /// The LLMs in this swarm, with optional counts and fallbacks.
    pub agents: Vec<agent::InlineAgentBaseWithFallbacksOrRemoteWithCount>,
    /// Optional weights for each agent. If `None`, uniform weights are used.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub weights: Option<Weights>,
}

impl InlineSwarmBase {
    /// Validates and converts to an [`InlineSwarm`] with computed ID.
    ///
    /// If `weights` is `None`, uniform weights (`Decimal::ONE` per agent) are used.
    /// Remote agent references are resolved from the provided hashmap.
    pub fn convert(
        self,
        remote_agents: Option<
            &HashMap<String, agent::RemoteAgentBaseWithFallbacks>,
        >,
    ) -> Result<InlineSwarm, String> {
        convert_base(self.agents, self.weights, remote_agents)
    }
}

/// A remote swarm base definition with metadata (without computed ID).
///
/// Like [`InlineSwarmBase`] but includes a description for remote storage.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "swarm.RemoteSwarmBase")]
pub struct RemoteSwarmBase {
    /// Human-readable description of what this swarm does.
    pub description: String,
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<InlineSwarmBase>")]
    pub inner: InlineSwarmBase,
}

impl RemoteSwarmBase {
    /// Validates and converts to a [`RemoteSwarm`] with computed ID.
    pub fn convert(
        self,
        remote_agents: Option<
            &HashMap<String, agent::RemoteAgentBaseWithFallbacks>,
        >,
    ) -> Result<RemoteSwarm, String> {
        Ok(RemoteSwarm {
            description: self.description,
            inner: self.inner.convert(remote_agents)?,
        })
    }
}

/// A swarm base definition, either remote (with metadata) or inline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "swarm.SwarmBase")]
pub enum SwarmBase {
    #[schemars(title = "Remote")]
    Remote(RemoteSwarmBase),
    #[schemars(title = "Inline")]
    Inline(InlineSwarmBase),
}

impl SwarmBase {
    /// Validates and converts to a [`Swarm`] with computed ID.
    pub fn convert(
        self,
        remote_agents: Option<
            &HashMap<String, agent::RemoteAgentBaseWithFallbacks>,
        >,
    ) -> Result<Swarm, String> {
        match self {
            SwarmBase::Remote(r) => {
                Ok(Swarm::Remote(r.convert(remote_agents)?))
            }
            SwarmBase::Inline(i) => {
                Ok(Swarm::Inline(i.convert(remote_agents)?))
            }
        }
    }
}

// ── Post-validation types (with computed ID) ───────────────────────

/// A validated inline Swarm with its computed content-addressed ID.
///
/// Created by converting from [`InlineSwarmBase`] via [`InlineSwarmBase::convert`].
/// The conversion:
/// 1. Validates and normalizes each agent
/// 2. Merges duplicate LLMs (by full_id) and sums their counts
/// 3. Sorts LLMs by full_id for deterministic ordering
/// 4. Computes the swarm ID from the sorted (full_id, count) pairs
/// 5. Aligns weights (merging duplicates by weighted average)
///
/// # Constraints
///
/// - Individual LLMs with `count: 0` are skipped
/// - Total agent count (sum of all counts) must be between 1 and 128
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "swarm.InlineSwarm")]
pub struct InlineSwarm {
    /// The deterministic content-addressed ID (22-character base62 string).
    pub id: String,
    /// The validated and deduplicated LLMs, sorted by full_id.
    pub agents: Vec<agent::AgentWithFallbacksWithCount>,
    /// The aligned weights for each agent.
    pub weights: Weights,
}

/// A validated remote Swarm with metadata and computed content-addressed ID.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "swarm.RemoteSwarm")]
pub struct RemoteSwarm {
    pub description: String,
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<InlineSwarm>")]
    pub inner: InlineSwarm,
}

/// A validated Swarm, either remote (with metadata) or inline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "swarm.Swarm")]
pub enum Swarm {
    #[schemars(title = "Remote")]
    Remote(RemoteSwarm),
    #[schemars(title = "Inline")]
    Inline(InlineSwarm),
}

impl InlineSwarm {
    /// Converts back to an `InlineSwarmBase`, dropping the computed ID.
    pub fn into_base(self) -> InlineSwarmBase {
        InlineSwarmBase {
            agents: self
                .agents
                .into_iter()
                .map(|a| agent::InlineAgentBaseWithFallbacksOrRemoteWithCount {
                    count: a.count,
                    inner:
                        agent::InlineAgentBaseWithFallbacksOrRemote::AgentBase(
                            match a.inner {
                                agent::AgentWithFallbacks::Inline(i) => {
                                    agent::InlineAgentBaseWithFallbacks {
                                        inner: i.inner.into_base(),
                                        fallbacks: i.fallbacks.map(|fbs| {
                                            fbs.into_iter()
                                                .map(|fb| fb.into_base())
                                                .collect()
                                        }),
                                    }
                                }
                                agent::AgentWithFallbacks::Remote(r) => {
                                    agent::InlineAgentBaseWithFallbacks {
                                        inner: r.inner.inner.into_base(),
                                        fallbacks: r.inner.fallbacks.map(
                                            |fbs| {
                                                fbs.into_iter()
                                                    .map(|fb| fb.into_base())
                                                    .collect()
                                            },
                                        ),
                                    }
                                }
                            },
                        ),
                })
                .collect(),
            weights: Some(self.weights),
        }
    }
}

impl Swarm {
    /// Returns the inner `InlineSwarm` regardless of variant.
    pub fn inline(&self) -> &InlineSwarm {
        match self {
            Swarm::Remote(r) => &r.inner,
            Swarm::Inline(i) => i,
        }
    }

    /// Consumes self and returns the inner `InlineSwarm`.
    pub fn into_inline(self) -> InlineSwarm {
        match self {
            Swarm::Remote(r) => r.inner,
            Swarm::Inline(i) => i,
        }
    }
}

// ── InlineSwarmBaseOrRemote ────────────────────────────────────────

/// A swarm specification that is either an inline swarm base
/// or a remote path reference.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "swarm.InlineSwarmBaseOrRemote")]
pub enum InlineSwarmBaseOrRemote {
    #[schemars(title = "SwarmBase")]
    SwarmBase(InlineSwarmBase),
    #[schemars(title = "Remote")]
    Remote(crate::RemotePath),
    /// A bare favorite name. Resolves at handler time.
    #[schemars(title = "Favorite")]
    Favorite(String),
}

/// Like [`InlineSwarmBaseOrRemote`] but with optional commit.
/// Used in request types where commit resolution happens server-side.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "swarm.InlineSwarmBaseOrRemoteCommitOptional")]
pub enum InlineSwarmBaseOrRemoteCommitOptional {
    #[schemars(title = "SwarmBase")]
    SwarmBase(InlineSwarmBase),
    #[schemars(title = "Remote")]
    Remote(crate::RemotePathCommitOptional),
    /// A bare favorite name. Resolves at handler time.
    #[schemars(title = "Favorite")]
    Favorite(String),
}

// ── Private helpers ────────────────────────────────────────────────

/// Validates agent fallbacks for duplicate IDs.
fn validate_agent_fallbacks(
    agent: &agent::AgentWithFallbacks,
) -> Result<(), String> {
    let inline = match agent {
        agent::AgentWithFallbacks::Remote(a) => &a.inner,
        agent::AgentWithFallbacks::Inline(a) => a,
    };
    if let Some(fallbacks) = &inline.fallbacks {
        if fallbacks.iter().any(|fb| fb.id() == inline.inner.id()) {
            return Err(format!(
                "Agent cannot have identical primary and fallback IDs: {}",
                inline.inner.id()
            ));
        }
        for i in 0..fallbacks.len() {
            for j in (i + 1)..fallbacks.len() {
                if fallbacks[i].id() == fallbacks[j].id() {
                    return Err(format!(
                        "Agent cannot have duplicate fallback IDs: {}",
                        fallbacks[i].id()
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Converts an agent slot (inline or remote reference) to a validated agent.
fn convert_agent_slot(
    slot: agent::InlineAgentBaseWithFallbacksOrRemote,
    remote_agents: Option<
        &HashMap<String, agent::RemoteAgentBaseWithFallbacks>,
    >,
) -> Result<agent::AgentWithFallbacks, String> {
    match slot {
        agent::InlineAgentBaseWithFallbacksOrRemote::AgentBase(
            base_with_fallbacks,
        ) => Ok(agent::AgentWithFallbacks::Inline(
            base_with_fallbacks.convert()?,
        )),
        agent::InlineAgentBaseWithFallbacksOrRemote::Remote(path) => {
            let key = path.key();
            let remote_agents = remote_agents.ok_or_else(|| {
                format!(
                    "remote agent reference '{}' but no agents hashmap provided",
                    key
                )
            })?;
            let agent_base = remote_agents.get(&key).ok_or_else(|| {
                format!("remote agent '{}' not found in agents hashmap", key)
            })?;
            Ok(agent::AgentWithFallbacks::Remote(
                agent_base.clone().convert()?,
            ))
        }
    }
}

/// Core conversion: validates agents, deduplicates, sorts, computes ID, aligns weights.
///
/// If `weights` is `None`, uniform weights (`Decimal::ONE` per agent) are used.
fn convert_base(
    agents: Vec<agent::InlineAgentBaseWithFallbacksOrRemoteWithCount>,
    weights: Option<Weights>,
    remote_agents: Option<
        &HashMap<String, agent::RemoteAgentBaseWithFallbacks>,
    >,
) -> Result<InlineSwarm, String> {
    // Resolve weights: use provided or default to uniform
    let weight_pairs: Vec<(Decimal, bool)> = match &weights {
        Some(w) => {
            if w.len() != agents.len() {
                return Err(format!(
                    "weights length ({}) does not match agents length ({})",
                    w.len(),
                    agents.len()
                ));
            }
            w.to_weights_and_invert()
        }
        None => vec![(Decimal::ONE, false); agents.len()],
    };

    // Validate weights are in [0, 1] and at least one is positive.
    let mut has_positive = false;
    for (i, (weight, _)) in weight_pairs.iter().enumerate() {
        if *weight < Decimal::ZERO || *weight > Decimal::ONE {
            return Err(format!(
                "weight at index {} must be between 0 and 1, got {}",
                i, weight
            ));
        }
        if *weight > Decimal::ZERO {
            has_positive = true;
        }
    }
    if !has_positive {
        return Err("weights must have at least one positive value".to_string());
    }

    let mut agents_with_full_id: IndexMap<
        String,
        (
            agent::AgentWithFallbacksWithCount,
            Decimal, // weighted sum
            u64,     // total count
            bool,    // invert
        ),
    > = IndexMap::with_capacity(agents.len());
    let mut count = 0u64;

    for (base_agent, (weight, invert)) in
        agents.into_iter().zip(weight_pairs.into_iter())
    {
        match base_agent.count {
            0 => continue,
            n => count += n,
        }
        let converted = convert_agent_slot(base_agent.inner, remote_agents)?;
        validate_agent_fallbacks(&converted)?;
        let full_id = converted.full_id();
        let agent_with_count = agent::AgentWithFallbacksWithCount {
            count: base_agent.count,
            inner: converted,
        };
        match agents_with_full_id.get_mut(&full_id) {
            Some((existing, weighted_sum, total_count, existing_invert)) => {
                if *existing_invert != invert {
                    return Err(format!(
                        "conflicting invert flags for merged agent with full_id: {}",
                        full_id
                    ));
                }
                *weighted_sum += weight * Decimal::from(agent_with_count.count);
                *total_count += agent_with_count.count;
                existing.count += agent_with_count.count;
            }
            None => {
                let weighted_sum =
                    weight * Decimal::from(agent_with_count.count);
                let total_count = agent_with_count.count;
                agents_with_full_id.insert(
                    full_id,
                    (agent_with_count, weighted_sum, total_count, invert),
                );
            }
        }
    }

    if count == 0 || count > 128 {
        return Err("`swarm.agents` must contain between 1 and 128 total LLMs"
            .to_string());
    }

    agents_with_full_id.sort_unstable_keys();

    let mut hasher = XxHash3_128::with_seed(0);
    for (full_id, (agent, _, _, _)) in &agents_with_full_id {
        hasher.write(full_id.as_bytes());
        let count_bytes = agent.count.to_le_bytes();
        hasher.write(&count_bytes);
    }
    let id = format!("{:0>22}", base62::encode(hasher.finish_128()));

    let mut result_agents = Vec::with_capacity(agents_with_full_id.len());
    let mut entries = Vec::with_capacity(agents_with_full_id.len());
    for (_, (agent, weighted_sum, total_count, invert)) in agents_with_full_id {
        result_agents.push(agent);
        let merged_weight = weighted_sum / Decimal::from(total_count);
        entries.push(WeightsEntry {
            weight: merged_weight,
            invert: if invert { Some(true) } else { None },
        });
    }

    Ok(InlineSwarm {
        id,
        agents: result_agents,
        weights: Weights::Entries(entries),
    })
}

/// Merge a validated agent into the dedup map.
fn merge_agent(
    agents_with_full_id: &mut IndexMap<
        String,
        agent::AgentWithFallbacksWithCount,
    >,
    agent_with_count: agent::AgentWithFallbacksWithCount,
) {
    let full_id = agent_with_count.inner.full_id();
    match agents_with_full_id.get_mut(&full_id) {
        Some(existing) => existing.count += agent_with_count.count,
        None => {
            agents_with_full_id.insert(full_id, agent_with_count);
        }
    }
}
