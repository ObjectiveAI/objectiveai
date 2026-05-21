use crate::tests::stream_push::stream_push_test;
use super::*;

fn completion(index: u64, error: Option<crate::error::ResponseError>) -> AgentCompletionChunk {
    AgentCompletionChunk {
        index,
        inner: crate::agent::completions::response::streaming::AgentCompletionChunk {
            id: format!("acc-{index}"),
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

fn chunk_with(
    completions: Vec<AgentCompletionChunk>,
    error: Option<crate::error::ResponseError>,
) -> FunctionInventionChunk {
    FunctionInventionChunk {
        id: "fic-ie".into(),
        completions,
        state: None,
        path: None,
        function: None,
        created: 0,
        object: Object::AlphaScalarFunctionInventionChunk,
        usage: None,
        error,
    }
}

fn err(code: u16, message: &str) -> crate::error::ResponseError {
    crate::error::ResponseError { code, message: message.into() }
}

#[test]
fn inner_errors_empty_completions() {
    let chunk = chunk_with(vec![], None);
    assert!(chunk.inner_errors().next().is_none());
}

#[test]
fn inner_errors_excludes_own_error() {
    let chunk = chunk_with(vec![], Some(err(500, "invention failed")));
    assert!(chunk.inner_errors().next().is_none());
}

#[test]
fn inner_errors_no_completion_errors() {
    let chunk = chunk_with(vec![completion(0, None), completion(1, None)], None);
    assert!(chunk.inner_errors().next().is_none());
}

#[test]
fn inner_errors_mixed() {
    let chunk = chunk_with(
        vec![
            completion(0, None),
            completion(1, Some(err(429, "rate limited"))),
            completion(2, None),
        ],
        None,
    );
    let collected: Vec<_> = chunk.inner_errors().collect();
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].agent_completion_index, 1);
    assert_eq!(collected[0].error.code, 429);
    assert_eq!(collected[0].error.message, serde_json::Value::String("rate limited".into()));
}

#[test]
fn inner_errors_all_completions_errored() {
    let chunk = chunk_with(
        vec![
            completion(0, Some(err(500, "a"))),
            completion(1, Some(err(502, "b"))),
            completion(2, Some(err(503, "c"))),
        ],
        None,
    );
    let collected: Vec<_> = chunk.inner_errors().collect();
    assert_eq!(collected.len(), 3);
    assert_eq!(collected[0].agent_completion_index, 0);
    assert_eq!(collected[0].error.code, 500);
    assert_eq!(collected[1].agent_completion_index, 1);
    assert_eq!(collected[1].error.code, 502);
    assert_eq!(collected[2].agent_completion_index, 2);
    assert_eq!(collected[2].error.code, 503);
}

#[test]
fn inner_error_serde_roundtrip() {
    let chunk = chunk_with(vec![completion(9, Some(err(404, "missing")))], None);
    let item = chunk.inner_errors().next().unwrap();
    let wire = serde_json::to_string(&item).unwrap();
    assert_eq!(wire, r#"{"agent_completion_index":9,"error":{"code":404,"message":"missing"}}"#);
    let round: InnerError<'static> = serde_json::from_str(&wire).unwrap();
    assert_eq!(round.agent_completion_index, 9);
    assert_eq!(round.error.code, 404);
    assert_eq!(round.error.message, serde_json::Value::String("missing".into()));
}

stream_push_test!(
    single_chunk_unchanged,
    vec![FunctionInventionChunk {
        id: "fic-1".into(),
        completions: vec![],
        state: None,
        path: None,
        function: None,
        created: 100,
        object: Object::AlphaScalarFunctionInventionChunk,
        usage: None,
        error: None,
    }],
    FunctionInventionChunk {
        id: "fic-1".into(),
        completions: vec![],
        state: None,
        path: None,
        function: None,
        created: 100,
        object: Object::AlphaScalarFunctionInventionChunk,
        usage: None,
        error: None,
    }
);

stream_push_test!(
    completions_merged_by_index,
    vec![
        FunctionInventionChunk {
            id: "fic-2".into(),
            completions: vec![AgentCompletionChunk {
                index: 0,
                inner: crate::agent::completions::response::streaming::AgentCompletionChunk {
                    id: "acc-1".into(),
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
            state: None,
            path: None,
            function: None,
            created: 100,
            object: Object::AlphaScalarFunctionInventionChunk,
            usage: None,
            error: None,
        },
        FunctionInventionChunk {
            id: "fic-2".into(),
            completions: vec![AgentCompletionChunk {
                index: 1,
                inner: crate::agent::completions::response::streaming::AgentCompletionChunk {
                    id: "acc-2".into(),
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
            state: None,
            path: None,
            function: None,
            created: 100,
            object: Object::AlphaScalarFunctionInventionChunk,
            usage: None,
            error: None,
        },
    ],
    FunctionInventionChunk {
        id: "fic-2".into(),
        completions: vec![
            AgentCompletionChunk {
                index: 0,
                inner: crate::agent::completions::response::streaming::AgentCompletionChunk {
                    id: "acc-1".into(),
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
        state: None,
        path: None,
        function: None,
        created: 100,
        object: Object::AlphaScalarFunctionInventionChunk,
        usage: None,
        error: None,
    }
);

stream_push_test!(
    usage_set_from_later_chunk,
    vec![
        FunctionInventionChunk {
            id: "fic-3".into(),
            completions: vec![],
            state: None,
            path: None,
            function: None,
            created: 100,
            object: Object::AlphaVectorFunctionInventionChunk,
            usage: None,
            error: None,
        },
        FunctionInventionChunk {
            id: "fic-3".into(),
            completions: vec![],
            state: None,
            path: None,
            function: None,
            created: 100,
            object: Object::AlphaVectorFunctionInventionChunk,
            usage: Some(crate::agent::completions::response::Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
                completion_tokens_details: None,
                prompt_tokens_details: None,
                cost: rust_decimal::Decimal::new(1, 3),
                cost_details: None,
                total_cost: rust_decimal::Decimal::new(1, 3),
            }),
            error: None,
        },
    ],
    FunctionInventionChunk {
        id: "fic-3".into(),
        completions: vec![],
        state: None,
        path: None,
        function: None,
        created: 100,
        object: Object::AlphaVectorFunctionInventionChunk,
        usage: Some(crate::agent::completions::response::Usage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
            completion_tokens_details: None,
            prompt_tokens_details: None,
            cost: rust_decimal::Decimal::new(1, 3),
            cost_details: None,
            total_cost: rust_decimal::Decimal::new(1, 3),
        }),
        error: None,
    }
);

stream_push_test!(
    error_replaced_by_later_chunk,
    vec![
        FunctionInventionChunk {
            id: "fic-4".into(),
            completions: vec![],
            state: None,
            path: None,
            function: None,
            created: 100,
            object: Object::AlphaScalarFunctionInventionChunk,
            usage: None,
            error: Some(crate::error::ResponseError {
                code: 500,
                message: serde_json::json!("first"),
            }),
        },
        FunctionInventionChunk {
            id: "fic-4".into(),
            completions: vec![],
            state: None,
            path: None,
            function: None,
            created: 100,
            object: Object::AlphaScalarFunctionInventionChunk,
            usage: None,
            error: Some(crate::error::ResponseError {
                code: 502,
                message: serde_json::json!("second"),
            }),
        },
    ],
    FunctionInventionChunk {
        id: "fic-4".into(),
        completions: vec![],
        state: None,
        path: None,
        function: None,
        created: 100,
        object: Object::AlphaScalarFunctionInventionChunk,
        usage: None,
        error: Some(crate::error::ResponseError {
            code: 502,
            message: serde_json::json!("second"),
        }),
    }
);

stream_push_test!(
    path_replaced_by_later_chunk,
    vec![
        FunctionInventionChunk {
            id: "fic-5".into(),
            completions: vec![],
            state: None,
            path: Some(crate::RemotePath::Github {
                owner: "owner-a".into(),
                repository: "repo-a".into(),
                commit: "aaa111".into(),
            }),
            function: None,
            created: 100,
            object: Object::AlphaScalarFunctionInventionChunk,
            usage: None,
            error: None,
        },
        FunctionInventionChunk {
            id: "fic-5".into(),
            completions: vec![],
            state: None,
            path: Some(crate::RemotePath::Github {
                owner: "owner-b".into(),
                repository: "repo-b".into(),
                commit: "abc123".into(),
            }),
            function: None,
            created: 100,
            object: Object::AlphaScalarFunctionInventionChunk,
            usage: None,
            error: None,
        },
    ],
    FunctionInventionChunk {
        id: "fic-5".into(),
        completions: vec![],
        state: None,
        path: Some(crate::RemotePath::Github {
            owner: "owner-b".into(),
            repository: "repo-b".into(),
            commit: "abc123".into(),
        }),
        function: None,
        created: 100,
        object: Object::AlphaScalarFunctionInventionChunk,
        usage: None,
        error: None,
    }
);
