use super::*;
use crate::tests::stream_push::stream_push_test;

fn completion(
    index: u64,
    error: Option<crate::error::ResponseError>,
) -> AgentCompletionChunk {
    AgentCompletionChunk {
        index,
        inner: crate::agent::completions::response::streaming::AgentCompletionChunk {
            id: format!("acc-{index}"),
            agent_instance_hierarchy: String::new(),
            agent_id: String::new(),
            agent_full_id: String::new(),
            agent_remote: None,
            created: 0,
            messages: vec![],
            object: crate::agent::completions::response::streaming::Object::AgentCompletionChunk,
            usage: None,
            upstream: crate::agent::Upstream::Openrouter,
            error,
            continuation: None,
            messages_queued: None,
        },
    }
}

fn chunk_with(completions: Vec<AgentCompletionChunk>) -> VectorCompletionChunk {
    VectorCompletionChunk {
        id: "vcc-ie".into(),
        completions,
        votes: vec![],
        scores: vec![],
        weights: vec![],
        created: 0,
        swarm: "ens-1".into(),
        object: Object::VectorCompletionChunk,
        usage: None,
    }
}

fn err(code: u16, message: &str) -> crate::error::ResponseError {
    crate::error::ResponseError {
        code,
        message: message.into(),
    }
}

#[test]
fn inner_errors_empty_completions() {
    let chunk = chunk_with(vec![]);
    assert!(chunk.inner_errors().next().is_none());
}

#[test]
fn inner_errors_no_errors() {
    let chunk = chunk_with(vec![completion(0, None), completion(1, None)]);
    assert!(chunk.inner_errors().next().is_none());
}

#[test]
fn inner_errors_single_error_at_index_2() {
    let chunk = chunk_with(vec![
        completion(0, None),
        completion(1, None),
        completion(2, Some(err(429, "rate limited"))),
    ]);
    let collected: Vec<_> = chunk.inner_errors().collect();
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].agent_completion_index, 2);
    assert_eq!(collected[0].error.code, 429);
    assert_eq!(
        collected[0].error.message,
        serde_json::Value::String("rate limited".into())
    );
}

#[test]
fn inner_error_serde_roundtrip() {
    let chunk = chunk_with(vec![completion(7, Some(err(503, "unavailable")))]);
    let item = chunk.inner_errors().next().expect("one inner error");
    let wire = serde_json::to_string(&item).unwrap();
    assert_eq!(
        wire,
        r#"{"agent_completion_index":7,"error":{"code":503,"message":"unavailable"}}"#
    );
    let round: InnerError<'static> = serde_json::from_str(&wire).unwrap();
    assert_eq!(round.agent_completion_index, 7);
    assert_eq!(round.error.code, 503);
    assert_eq!(
        round.error.message,
        serde_json::Value::String("unavailable".into())
    );
}

#[test]
fn inner_errors_all_completions_errored() {
    let chunk = chunk_with(vec![
        completion(0, Some(err(500, "a"))),
        completion(1, Some(err(502, "b"))),
        completion(2, Some(err(503, "c"))),
    ]);
    let collected: Vec<_> = chunk.inner_errors().collect();
    assert_eq!(collected.len(), 3);
    assert_eq!(collected[0].agent_completion_index, 0);
    assert_eq!(collected[0].error.code, 500);
    assert_eq!(collected[1].agent_completion_index, 1);
    assert_eq!(collected[1].error.code, 502);
    assert_eq!(collected[2].agent_completion_index, 2);
    assert_eq!(collected[2].error.code, 503);
}

stream_push_test!(
    single_chunk_unchanged,
    vec![VectorCompletionChunk {
        id: "vcc-1".into(),
        completions: vec![],
        votes: vec![],
        scores: vec![],
        weights: vec![],
        created: 100,
        swarm: "ens-1".into(),
        object: Object::VectorCompletionChunk,
        usage: None,
    }],
    VectorCompletionChunk {
        id: "vcc-1".into(),
        completions: vec![],
        votes: vec![],
        scores: vec![],
        weights: vec![],
        created: 100,
        swarm: "ens-1".into(),
        object: Object::VectorCompletionChunk,
        usage: None,
    }
);

stream_push_test!(
    scores_and_weights_replaced,
    vec![
        VectorCompletionChunk {
            id: "vcc-2".into(),
            completions: vec![],
            votes: vec![],
            scores: vec![],
            weights: vec![],
            created: 100,
            swarm: "ens-1".into(),
            object: Object::VectorCompletionChunk,
            usage: None,
        },
        VectorCompletionChunk {
            id: "vcc-2".into(),
            completions: vec![],
            votes: vec![],
            scores: vec![
                rust_decimal::Decimal::new(60, 2),
                rust_decimal::Decimal::new(40, 2),
            ],
            weights: vec![
                rust_decimal::Decimal::new(3, 1),
                rust_decimal::Decimal::new(7, 1),
            ],
            created: 100,
            swarm: "ens-1".into(),
            object: Object::VectorCompletionChunk,
            usage: None,
        },
    ],
    VectorCompletionChunk {
        id: "vcc-2".into(),
        completions: vec![],
        votes: vec![],
        scores: vec![
            rust_decimal::Decimal::new(60, 2),
            rust_decimal::Decimal::new(40, 2),
        ],
        weights: vec![
            rust_decimal::Decimal::new(3, 1),
            rust_decimal::Decimal::new(7, 1),
        ],
        created: 100,
        swarm: "ens-1".into(),
        object: Object::VectorCompletionChunk,
        usage: None,
    }
);

stream_push_test!(
    completions_merged_by_index,
    vec![
        VectorCompletionChunk {
            id: "vcc-3".into(),
            completions: vec![AgentCompletionChunk {
                index: 0,
                inner: crate::agent::completions::response::streaming::AgentCompletionChunk {
                    id: "acc-1".into(),
                    agent_instance_hierarchy: String::new(),
                    agent_id: String::new(),
                    agent_full_id: String::new(),
                    agent_remote: None,
                    created: 0,
                    messages: vec![],
                    object: crate::agent::completions::response::streaming::Object::AgentCompletionChunk,
                    usage: None,
                    upstream: crate::agent::Upstream::Openrouter,
                    error: None,
                    continuation: None,
                    messages_queued: None,
                },
            }],
            votes: vec![],
            scores: vec![],
            weights: vec![],
            created: 100,
            swarm: "ens-1".into(),
            object: Object::VectorCompletionChunk,
            usage: None,
        },
        VectorCompletionChunk {
            id: "vcc-3".into(),
            completions: vec![AgentCompletionChunk {
                index: 1,
                inner: crate::agent::completions::response::streaming::AgentCompletionChunk {
                    id: "acc-2".into(),
                    agent_instance_hierarchy: String::new(),
                    agent_id: String::new(),
                    agent_full_id: String::new(),
                    agent_remote: None,
                    created: 0,
                    messages: vec![],
                    object: crate::agent::completions::response::streaming::Object::AgentCompletionChunk,
                    usage: None,
                    upstream: crate::agent::Upstream::Openrouter,
                    error: None,
                    continuation: None,
                    messages_queued: None,
                },
            }],
            votes: vec![],
            scores: vec![],
            weights: vec![],
            created: 100,
            swarm: "ens-1".into(),
            object: Object::VectorCompletionChunk,
            usage: None,
        },
    ],
    VectorCompletionChunk {
        id: "vcc-3".into(),
        completions: vec![
            AgentCompletionChunk {
                index: 0,
                inner: crate::agent::completions::response::streaming::AgentCompletionChunk {
                    id: "acc-1".into(),
                    agent_instance_hierarchy: String::new(),
                    agent_id: String::new(),
                    agent_full_id: String::new(),
                    agent_remote: None,
                    created: 0,
                    messages: vec![],
                    object: crate::agent::completions::response::streaming::Object::AgentCompletionChunk,
                    usage: None,
                    upstream: crate::agent::Upstream::Openrouter,
                    error: None,
                    continuation: None,
                    messages_queued: None,
                },
            },
            AgentCompletionChunk {
                index: 1,
                inner: crate::agent::completions::response::streaming::AgentCompletionChunk {
                    id: "acc-2".into(),
                    agent_instance_hierarchy: String::new(),
                    agent_id: String::new(),
                    agent_full_id: String::new(),
                    agent_remote: None,
                    created: 0,
                    messages: vec![],
                    object: crate::agent::completions::response::streaming::Object::AgentCompletionChunk,
                    usage: None,
                    upstream: crate::agent::Upstream::Openrouter,
                    error: None,
                    continuation: None,
                    messages_queued: None,
                },
            },
        ],
        votes: vec![],
        scores: vec![],
        weights: vec![],
        created: 100,
        swarm: "ens-1".into(),
        object: Object::VectorCompletionChunk,
        usage: None,
    }
);

stream_push_test!(
    votes_appended,
    vec![
        VectorCompletionChunk {
            id: "vcc-4".into(),
            completions: vec![],
            votes: vec![crate::vector::completions::response::Vote {
                agent: String::new(),
                swarm_index: 0,
                flat_swarm_index: 0,
                prompt_id: "p1".into(),
                responses_ids: vec!["r1".into()],
                vote: vec![rust_decimal::Decimal::ONE],
                weight: rust_decimal::Decimal::ONE,
                retry: None,
                from_cache: None,
                completion_index: None,
            }],
            scores: vec![],
            weights: vec![],
            created: 100,
            swarm: "ens-1".into(),
            object: Object::VectorCompletionChunk,
            usage: None,
        },
        VectorCompletionChunk {
            id: "vcc-4".into(),
            completions: vec![],
            votes: vec![crate::vector::completions::response::Vote {
                agent: String::new(),
                swarm_index: 1,
                flat_swarm_index: 1,
                prompt_id: "p1".into(),
                responses_ids: vec!["r1".into()],
                vote: vec![rust_decimal::Decimal::ONE],
                weight: rust_decimal::Decimal::ONE,
                retry: None,
                from_cache: Some(true),
                completion_index: None,
            }],
            scores: vec![],
            weights: vec![],
            created: 100,
            swarm: "ens-1".into(),
            object: Object::VectorCompletionChunk,
            usage: None,
        },
    ],
    VectorCompletionChunk {
        id: "vcc-4".into(),
        completions: vec![],
        votes: vec![
            crate::vector::completions::response::Vote {
                agent: String::new(),
                swarm_index: 0,
                flat_swarm_index: 0,
                prompt_id: "p1".into(),
                responses_ids: vec!["r1".into()],
                vote: vec![rust_decimal::Decimal::ONE],
                weight: rust_decimal::Decimal::ONE,
                retry: None,
                from_cache: None,
                completion_index: None,
            },
            crate::vector::completions::response::Vote {
                agent: String::new(),
                swarm_index: 1,
                flat_swarm_index: 1,
                prompt_id: "p1".into(),
                responses_ids: vec!["r1".into()],
                vote: vec![rust_decimal::Decimal::ONE],
                weight: rust_decimal::Decimal::ONE,
                retry: None,
                from_cache: Some(true),
                completion_index: None,
            },
        ],
        scores: vec![],
        weights: vec![],
        created: 100,
        swarm: "ens-1".into(),
        object: Object::VectorCompletionChunk,
        usage: None,
    }
);

stream_push_test!(
    usage_set_from_later_chunk,
    vec![
        VectorCompletionChunk {
            id: "vcc-5".into(),
            completions: vec![],
            votes: vec![],
            scores: vec![],
            weights: vec![],
            created: 100,
            swarm: "ens-1".into(),
            object: Object::VectorCompletionChunk,
            usage: None,
        },
        VectorCompletionChunk {
            id: "vcc-5".into(),
            completions: vec![],
            votes: vec![],
            scores: vec![],
            weights: vec![],
            created: 100,
            swarm: "ens-1".into(),
            object: Object::VectorCompletionChunk,
            usage: Some(crate::agent::completions::response::Usage {
                prompt_tokens: 20,
                completion_tokens: 10,
                total_tokens: 30,
                completion_tokens_details: None,
                prompt_tokens_details: None,
                cost: rust_decimal::Decimal::ZERO,
                cost_details: None,
                total_cost: rust_decimal::Decimal::ZERO,
            }),
        },
    ],
    VectorCompletionChunk {
        id: "vcc-5".into(),
        completions: vec![],
        votes: vec![],
        scores: vec![],
        weights: vec![],
        created: 100,
        swarm: "ens-1".into(),
        object: Object::VectorCompletionChunk,
        usage: Some(crate::agent::completions::response::Usage {
            prompt_tokens: 20,
            completion_tokens: 10,
            total_tokens: 30,
            completion_tokens_details: None,
            prompt_tokens_details: None,
            cost: rust_decimal::Decimal::ZERO,
            cost_details: None,
            total_cost: rust_decimal::Decimal::ZERO,
        }),
    }
);
