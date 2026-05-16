use super::*;
use crate::agent::{self, InlineAgentBaseWithFallbacksOrRemote, InlineAgentBaseWithFallbacksOrRemoteWithCount};

use crate::weights::{Weights, WeightsEntry};
use rust_decimal::dec;
use rust_decimal::Decimal;

fn make_agent(model: &str, count: u64) -> InlineAgentBaseWithFallbacksOrRemoteWithCount {
    InlineAgentBaseWithFallbacksOrRemoteWithCount {
        count,
        inner: InlineAgentBaseWithFallbacksOrRemote::AgentBase(agent::InlineAgentBaseWithFallbacks {
            inner: agent::InlineAgentBase::Openrouter(agent::openrouter::AgentBase {
                model: model.to_string(),
                ..Default::default()
            }),
            fallbacks: None,
        }),
    }
}

/// Helper: extract the model name from a validated AgentWithFallbacks.
fn agent_model(a: &agent::AgentWithFallbacks) -> &str {
    match a {
        agent::AgentWithFallbacks::Inline(wf) => wf.inner.base().model(),
        agent::AgentWithFallbacks::Remote(wf) => wf.inner.inner.base().model(),
    }
}

fn make_or(model: &str) -> agent::InlineAgentBase {
    agent::InlineAgentBase::Openrouter(agent::openrouter::AgentBase {
        model: model.to_string(),
        ..Default::default()
    })
}

#[test]
fn filter_removes_count_zero_llms_and_profile_entries() {
    let base = InlineSwarmBase {
        weights: None,
        agents: vec![
            make_agent("openai/gpt-4o", 1),
            make_agent("openai/gpt-4o-mini", 0), // should be filtered
            make_agent("anthropic/claude-3.5-sonnet", 1),
        ],
    };
    let profile = Weights::Entries(vec![
        WeightsEntry { weight: dec!(0.6), invert: None },
        WeightsEntry { weight: dec!(0.9), invert: None }, // should be filtered
        WeightsEntry { weight: dec!(0.4), invert: None },
    ]);

    let result = InlineSwarmBase { agents: base.agents, weights: Some(profile) }.convert(None).unwrap();
    let swarm = &result;
    let aligned = &result.weights;
    assert_eq!(swarm.agents.len(), 2);
    let pairs = aligned.to_weights_and_invert();
    assert_eq!(pairs.len(), 2);
    // The two remaining weights should be 0.6 and 0.4 (possibly reordered by sort)
    let weights: Vec<Decimal> = pairs.iter().map(|(w, _)| *w).collect();
    assert!(weights.contains(&dec!(0.6)));
    assert!(weights.contains(&dec!(0.4)));
}

#[test]
fn merge_duplicates_with_weighted_average() {
    // Two identical LLMs: count 3 weight 1.0, count 1 weight 0.5
    // Merged weight = (1.0 * 3 + 0.5 * 1) / (3 + 1) = 3.5 / 4 = 0.875
    let base = InlineSwarmBase {
        weights: None,
        agents: vec![
            make_agent("openai/gpt-4o", 3),
            make_agent("openai/gpt-4o", 1),
        ],
    };
    let profile = Weights::Entries(vec![
        WeightsEntry { weight: dec!(1.0), invert: None },
        WeightsEntry { weight: dec!(0.5), invert: None },
    ]);

    let result = InlineSwarmBase { agents: base.agents, weights: Some(profile) }.convert(None).unwrap();
    let swarm = &result;
    let aligned = &result.weights;
    assert_eq!(swarm.agents.len(), 1);
    assert_eq!(swarm.agents[0].count, 4);
    let pairs = aligned.to_weights_and_invert();
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].0, dec!(0.875));
    assert_eq!(pairs[0].1, false);
}

#[test]
fn sort_reorders_profile_entries() {
    // Put two different LLMs in; verify profile entries follow the LLMs after sort
    let base = InlineSwarmBase {
        weights: None,
        agents: vec![
            make_agent("openai/gpt-4o", 1),
            make_agent("anthropic/claude-3.5-sonnet", 1),
        ],
    };
    let profile = Weights::Entries(vec![
        WeightsEntry { weight: dec!(0.7), invert: None },
        WeightsEntry { weight: dec!(0.3), invert: Some(true) },
    ]);

    let result = InlineSwarmBase { agents: base.agents, weights: Some(profile) }.convert(None).unwrap();
    let swarm = &result;
    let aligned = &result.weights;
    let pairs = aligned.to_weights_and_invert();
    assert_eq!(swarm.agents.len(), 2);

    // Sorted by full_id (content-addressed hash), not model name.
    // Verify that the profile entry with weight 0.7 follows the openai LLM
    // and the entry with weight 0.3 follows the anthropic LLM.
    for (i, a) in swarm.agents.iter().enumerate() {
        if agent_model(&a.inner) == "openai/gpt-4o" {
            assert_eq!(pairs[i].0, dec!(0.7));
            assert_eq!(pairs[i].1, false);
        } else {
            assert_eq!(agent_model(&a.inner), "anthropic/claude-3.5-sonnet");
            assert_eq!(pairs[i].0, dec!(0.3));
            assert_eq!(pairs[i].1, true);
        }
    }
}

#[test]
fn combined_filter_merge_sort() {
    let base = InlineSwarmBase {
        weights: None,
        agents: vec![
            make_agent("openai/gpt-4o", 2),
            make_agent("anthropic/claude-3.5-sonnet", 0), // filtered
            make_agent("openai/gpt-4o", 2),               // merged with first
            make_agent("google/gemini-2.0-flash", 1),
        ],
    };
    let profile = Weights::Entries(vec![
        WeightsEntry { weight: dec!(0.8), invert: None },
        WeightsEntry { weight: dec!(0.5), invert: None }, // filtered
        WeightsEntry { weight: dec!(0.4), invert: None }, // merged: (0.8*2 + 0.4*2)/4 = 0.6
        WeightsEntry { weight: dec!(0.9), invert: None },
    ]);

    let result = InlineSwarmBase { agents: base.agents, weights: Some(profile) }.convert(None).unwrap();
    let swarm = &result;
    let aligned = &result.weights;
    assert_eq!(swarm.agents.len(), 2);
    let pairs = aligned.to_weights_and_invert();
    assert_eq!(pairs.len(), 2);

    // sorted by full_id — check which order they ended up in
    for (i, a) in swarm.agents.iter().enumerate() {
        let model = agent_model(&a.inner);
        if model.starts_with("google") {
            assert_eq!(pairs[i].0, dec!(0.9)); // google weight
        } else {
            assert!(model.starts_with("openai"));
            assert_eq!(pairs[i].0, dec!(0.6)); // merged openai weight
        }
    }
}

#[test]
fn error_on_conflicting_invert_flags() {
    let base = InlineSwarmBase {
        weights: None,
        agents: vec![
            make_agent("openai/gpt-4o", 1),
            make_agent("openai/gpt-4o", 1),
        ],
    };
    let profile = Weights::Entries(vec![
        WeightsEntry { weight: dec!(0.5), invert: None },
        WeightsEntry { weight: dec!(0.5), invert: Some(true) },
    ]);

    let result = InlineSwarmBase { agents: base.agents, weights: Some(profile) }.convert(None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("conflicting invert flags"));
}

#[test]
fn error_on_profile_length_mismatch() {
    let base = InlineSwarmBase {
        weights: None,
        agents: vec![
            make_agent("openai/gpt-4o", 1),
            make_agent("anthropic/claude-3.5-sonnet", 1),
        ],
    };
    let profile = Weights::Entries(vec![
        WeightsEntry { weight: dec!(0.5), invert: None },
    ]);

    let result = InlineSwarmBase { agents: base.agents, weights: Some(profile) }.convert(None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("does not match"));
}

#[test]
fn legacy_weights_format() {
    let base = InlineSwarmBase {
        weights: None,
        agents: vec![
            make_agent("openai/gpt-4o", 1),
            make_agent("anthropic/claude-3.5-sonnet", 1),
        ],
    };
    let profile = Weights::Weights(vec![dec!(0.7), dec!(0.3)]);

    let result = InlineSwarmBase { agents: base.agents, weights: Some(profile) }.convert(None).unwrap();
    let swarm = &result;
    let aligned = &result.weights;
    assert_eq!(swarm.agents.len(), 2);
    let pairs = aligned.to_weights_and_invert();
    assert_eq!(pairs.len(), 2);
    // All inverts should be false for Weights format
    assert_eq!(pairs[0].1, false);
    assert_eq!(pairs[1].1, false);
}

#[test]
fn produces_same_swarm_id_as_convert() {
    let base = InlineSwarmBase {
        weights: None,
        agents: vec![
            make_agent("openai/gpt-4o", 2),
            make_agent("anthropic/claude-3.5-sonnet", 1),
            make_agent("openai/gpt-4o", 1), // duplicate, will be merged
        ],
    };
    let profile = Weights::Weights(vec![dec!(0.5), dec!(0.5), dec!(0.5)]);

    let swarm_from_convert: InlineSwarm = base.clone().convert(None).unwrap();
    let result = InlineSwarmBase { agents: base.agents, weights: Some(profile) }.convert(None).unwrap();
    let swarm_with_profile = result;

    assert_eq!(swarm_from_convert.id, swarm_with_profile.id);
    assert_eq!(swarm_from_convert.agents.len(), swarm_with_profile.agents.len());
    for (a, b) in swarm_from_convert.agents.iter().zip(swarm_with_profile.agents.iter()) {
        assert_eq!(a.count, b.count);
        assert_eq!(a.inner.full_id(), b.inner.full_id());
    }
}

// ---- Parity tests: convert and WithWeights::convert must always produce
//      identical swarms (same id, same agent order, same counts). ----

/// Helper: assert that InlineSwarmBase::convert and WithWeights::convert
/// produce identical swarms for the given base.
fn assert_parity(base: InlineSwarmBase) {
    let n = base.agents.len();
    // uniform weights so profile is always valid
    let profile = Weights::Weights(vec![dec!(0.5); n]);

    let swarm_convert: InlineSwarm = base.clone().convert(None).unwrap();
    let swarm_wp = InlineSwarmBase { agents: base.agents, weights: Some(profile) }.convert(None).unwrap();

    assert_eq!(swarm_convert.id, swarm_wp.id,
        "IDs differ: convert={}, with_profile={}", swarm_convert.id, swarm_wp.id);
    assert_eq!(swarm_convert.agents.len(), swarm_wp.agents.len(),
        "agent count differs");
    for (a, b) in swarm_convert.agents.iter().zip(swarm_wp.agents.iter()) {
        assert_eq!(a.count, b.count, "count differs for full_id {}", a.inner.full_id());
        assert_eq!(a.inner.full_id(), b.inner.full_id(), "full_id differs");
        assert_eq!(agent_model(&a.inner), agent_model(&b.inner), "model differs");
    }
}

#[test]
fn parity_single_llm() {
    assert_parity(InlineSwarmBase {
        weights: None,
        agents: vec![make_agent("openai/gpt-4o", 1)],
    });
}

#[test]
fn parity_two_distinct_llms() {
    assert_parity(InlineSwarmBase {
        weights: None,
        agents: vec![
            make_agent("openai/gpt-4o", 1),
            make_agent("anthropic/claude-3.5-sonnet", 1),
        ],
    });
}

#[test]
fn parity_many_distinct_models() {
    assert_parity(InlineSwarmBase {
        weights: None,
        agents: vec![
            make_agent("openai/gpt-4o", 1),
            make_agent("anthropic/claude-3.5-sonnet", 2),
            make_agent("google/gemini-2.0-flash", 1),
            make_agent("meta/llama-3-70b", 3),
            make_agent("mistral/mixtral-8x7b", 1),
        ],
    });
}

#[test]
fn parity_all_same_model_merged() {
    // 4 entries for the same model -> merged into 1 with count=10
    assert_parity(InlineSwarmBase {
        weights: None,
        agents: vec![
            make_agent("openai/gpt-4o", 3),
            make_agent("openai/gpt-4o", 2),
            make_agent("openai/gpt-4o", 4),
            make_agent("openai/gpt-4o", 1),
        ],
    });
}

#[test]
fn parity_with_count_zero_filtered() {
    assert_parity(InlineSwarmBase {
        weights: None,
        agents: vec![
            make_agent("openai/gpt-4o", 0),       // filtered
            make_agent("anthropic/claude-3.5-sonnet", 1),
            make_agent("google/gemini-2.0-flash", 0), // filtered
            make_agent("meta/llama-3-70b", 2),
        ],
    });
}

#[test]
fn parity_interleaved_duplicates_and_zeros() {
    // Mix of count-0 filtering and duplicate merging
    assert_parity(InlineSwarmBase {
        weights: None,
        agents: vec![
            make_agent("openai/gpt-4o", 2),
            make_agent("anthropic/claude-3.5-sonnet", 0), // filtered
            make_agent("openai/gpt-4o", 3),               // merged -> count 5
            make_agent("google/gemini-2.0-flash", 1),
            make_agent("google/gemini-2.0-flash", 0),     // filtered (count 0)
            make_agent("meta/llama-3-70b", 1),
            make_agent("openai/gpt-4o", 1),               // merged -> count 6
        ],
    });
}

#[test]
fn parity_different_output_modes_are_distinct() {
    use agent::openrouter;
    // Same model but different output_mode -> different full_id, not merged
    let base = InlineSwarmBase {
        weights: None,
        agents: vec![
            InlineAgentBaseWithFallbacksOrRemoteWithCount {
                count: 1,
                inner: InlineAgentBaseWithFallbacksOrRemote::AgentBase(agent::InlineAgentBaseWithFallbacks {
                    inner: agent::InlineAgentBase::Openrouter(openrouter::AgentBase {
                        model: "openai/gpt-4o".to_string(),
                        output_mode: openrouter::OutputMode::Instruction,
                        ..Default::default()
                    }),
                    fallbacks: None,
                }),
            },
            InlineAgentBaseWithFallbacksOrRemoteWithCount {
                count: 1,
                inner: InlineAgentBaseWithFallbacksOrRemote::AgentBase(agent::InlineAgentBaseWithFallbacks {
                    inner: agent::InlineAgentBase::Openrouter(openrouter::AgentBase {
                        model: "openai/gpt-4o".to_string(),
                        output_mode: openrouter::OutputMode::JsonSchema,
                        ..Default::default()
                    }),
                    fallbacks: None,
                }),
            },
        ],
    };
    assert_parity(base);
}

#[test]
fn parity_different_temperatures_are_distinct() {
    use agent::openrouter;
    let base = InlineSwarmBase {
        weights: None,
        agents: vec![
            InlineAgentBaseWithFallbacksOrRemoteWithCount {
                count: 1,
                inner: InlineAgentBaseWithFallbacksOrRemote::AgentBase(agent::InlineAgentBaseWithFallbacks {
                    inner: agent::InlineAgentBase::Openrouter(openrouter::AgentBase {
                        model: "openai/gpt-4o".to_string(),
                        temperature: Some(0.0),
                        ..Default::default()
                    }),
                    fallbacks: None,
                }),
            },
            InlineAgentBaseWithFallbacksOrRemoteWithCount {
                count: 1,
                inner: InlineAgentBaseWithFallbacksOrRemote::AgentBase(agent::InlineAgentBaseWithFallbacks {
                    inner: agent::InlineAgentBase::Openrouter(openrouter::AgentBase {
                        model: "openai/gpt-4o".to_string(),
                        temperature: Some(1.5),
                        ..Default::default()
                    }),
                    fallbacks: None,
                }),
            },
        ],
    };
    assert_parity(base);
}

#[test]
fn parity_with_fallbacks() {
    let base = InlineSwarmBase {
        weights: None,
        agents: vec![
            InlineAgentBaseWithFallbacksOrRemoteWithCount {
                count: 2,
                inner: InlineAgentBaseWithFallbacksOrRemote::AgentBase(agent::InlineAgentBaseWithFallbacks {
                    inner: make_or("openai/gpt-4o"),
                    fallbacks: Some(vec![make_or("openai/gpt-4o-mini")]),
                }),
            },
            make_agent("anthropic/claude-3.5-sonnet", 1),
        ],
    };
    assert_parity(base);
}

#[test]
fn parity_duplicate_llms_with_fallbacks_merged() {
    // Two entries with same primary+fallback config -> should merge
    let make_with_fallback = || InlineAgentBaseWithFallbacksOrRemoteWithCount {
        count: 2,
        inner: InlineAgentBaseWithFallbacksOrRemote::AgentBase(agent::InlineAgentBaseWithFallbacks {
            inner: make_or("openai/gpt-4o"),
            fallbacks: Some(vec![make_or("openai/gpt-4o-mini")]),
        }),
    };
    assert_parity(InlineSwarmBase {
        weights: None,
        agents: vec![make_with_fallback(), make_with_fallback()],
    });
}

#[test]
fn parity_high_count() {
    // Max total count is 128
    assert_parity(InlineSwarmBase {
        weights: None,
        agents: vec![
            make_agent("openai/gpt-4o", 64),
            make_agent("anthropic/claude-3.5-sonnet", 64),
        ],
    });
}

#[test]
fn parity_single_llm_high_count() {
    assert_parity(InlineSwarmBase {
        weights: None,
        agents: vec![make_agent("openai/gpt-4o", 128)],
    });
}

#[test]
fn parity_many_duplicates_merged_to_one() {
    // 10 identical entries of count 1 -> merged to count 10
    assert_parity(InlineSwarmBase {
        weights: None,
        agents: (0..10).map(|_| make_agent("openai/gpt-4o", 1)).collect(),
    });
}

#[test]
fn parity_reverse_input_order() {
    // Verify that swapping input order still produces the same swarm
    let base_a = InlineSwarmBase {
        weights: None,
        agents: vec![
            make_agent("openai/gpt-4o", 1),
            make_agent("anthropic/claude-3.5-sonnet", 2),
            make_agent("google/gemini-2.0-flash", 3),
        ],
    };
    let base_b = InlineSwarmBase {
        weights: None,
        agents: vec![
            make_agent("google/gemini-2.0-flash", 3),
            make_agent("anthropic/claude-3.5-sonnet", 2),
            make_agent("openai/gpt-4o", 1),
        ],
    };

    let convert_a: InlineSwarm = base_a.clone().convert(None).unwrap();
    let convert_b: InlineSwarm = base_b.clone().convert(None).unwrap();
    assert_eq!(convert_a.id, convert_b.id, "convert should be order-independent");

    let profile_a = Weights::Weights(vec![dec!(0.5); 3]);
    let profile_b = Weights::Weights(vec![dec!(0.5); 3]);
    let wp_a = InlineSwarmBase { agents: base_a.agents, weights: Some(profile_a) }.convert(None).unwrap();
    let wp_b = InlineSwarmBase { agents: base_b.agents, weights: Some(profile_b) }.convert(None).unwrap();
    assert_eq!(wp_a.id, wp_b.id, "WithWeights::convert should be order-independent");
    assert_eq!(convert_a.id, wp_a.id, "convert and WithWeights::convert should match");
}

#[test]
fn parity_entries_profile_format() {
    // Use Entries format (not Weights) and verify parity
    let base = InlineSwarmBase {
        weights: None,
        agents: vec![
            make_agent("openai/gpt-4o", 2),
            make_agent("anthropic/claude-3.5-sonnet", 1),
            make_agent("openai/gpt-4o", 1),
        ],
    };
    let profile = Weights::Entries(vec![
        WeightsEntry { weight: dec!(0.8), invert: Some(true) },
        WeightsEntry { weight: dec!(0.3), invert: None },
        WeightsEntry { weight: dec!(0.2), invert: Some(true) },
    ]);

    let swarm_convert: InlineSwarm = base.clone().convert(None).unwrap();
    let swarm_wp = InlineSwarmBase { agents: base.agents, weights: Some(profile) }.convert(None).unwrap();

    assert_eq!(swarm_convert.id, swarm_wp.id);
    assert_eq!(swarm_convert.agents.len(), swarm_wp.agents.len());
    for (a, b) in swarm_convert.agents.iter().zip(swarm_wp.agents.iter()) {
        assert_eq!(a.count, b.count);
        assert_eq!(a.inner.full_id(), b.inner.full_id());
    }
}

#[test]
fn parity_all_but_one_filtered() {
    // All count-0 except one -> should produce single-agent swarm
    assert_parity(InlineSwarmBase {
        weights: None,
        agents: vec![
            make_agent("openai/gpt-4o", 0),
            make_agent("anthropic/claude-3.5-sonnet", 0),
            make_agent("google/gemini-2.0-flash", 0),
            make_agent("meta/llama-3-70b", 1),
        ],
    });
}

#[test]
fn parity_both_error_on_all_zero_counts() {
    let base = InlineSwarmBase {
        weights: None,
        agents: vec![
            make_agent("openai/gpt-4o", 0),
            make_agent("anthropic/claude-3.5-sonnet", 0),
        ],
    };
    let profile = Weights::Weights(vec![dec!(0.5), dec!(0.5)]);

    let convert_result: Result<InlineSwarm, _> = base.clone().convert(None);
    let wp_result = InlineSwarmBase { agents: base.agents, weights: Some(profile) }.convert(None);

    assert!(convert_result.is_err(), "convert should error on all-zero counts");
    assert!(wp_result.is_err(), "WithWeights::convert should error on all-zero counts");
}
