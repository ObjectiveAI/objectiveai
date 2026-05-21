use crate::tests::stream_push::stream_push_test;
use super::*;

fn agent_completion_wrapper(
    index: u64,
    error: Option<crate::error::ResponseError>,
) -> crate::functions::inventions::response::streaming::AgentCompletionChunk {
    crate::functions::inventions::response::streaming::AgentCompletionChunk {
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

fn non_recursive_invention(
    completions: Vec<crate::functions::inventions::response::streaming::AgentCompletionChunk>,
    own_error: Option<crate::error::ResponseError>,
) -> crate::functions::inventions::response::streaming::FunctionInventionChunk {
    crate::functions::inventions::response::streaming::FunctionInventionChunk {
        id: "fi-ie".into(),
        completions,
        state: None,
        path: None,
        function: None,
        created: 0,
        object: crate::functions::inventions::response::streaming::Object::AlphaScalarFunctionInventionChunk,
        usage: None,
        error: own_error,
    }
}

fn wrapper(
    index: u64,
    inner: crate::functions::inventions::response::streaming::FunctionInventionChunk,
) -> FunctionInventionChunk {
    FunctionInventionChunk { index, inner }
}

fn rec_chunk(inventions: Vec<FunctionInventionChunk>) -> FunctionInventionRecursiveChunk {
    FunctionInventionRecursiveChunk {
        id: "firc-ie".into(),
        inventions,
        inventions_errors: None,
        created: 0,
        object: Object::AlphaScalarFunctionInventionRecursiveChunk,
        usage: None,
    }
}

fn err(code: u16, message: &str) -> crate::error::ResponseError {
    crate::error::ResponseError { code, message: message.into() }
}

#[test]
fn inner_errors_empty_inventions() {
    let chunk = rec_chunk(vec![]);
    assert!(chunk.inner_errors().next().is_none());
}

#[test]
fn inner_errors_no_invention_errors() {
    let chunk = rec_chunk(vec![
        wrapper(0, non_recursive_invention(vec![agent_completion_wrapper(0, None)], None)),
        wrapper(1, non_recursive_invention(vec![], None)),
    ]);
    assert!(chunk.inner_errors().next().is_none());
}

#[test]
fn inner_errors_invention_own_error() {
    let chunk = rec_chunk(vec![
        wrapper(1, non_recursive_invention(
            vec![agent_completion_wrapper(0, None)],
            Some(err(500, "invention crashed")),
        )),
    ]);
    let collected: Vec<_> = chunk.inner_errors().collect();
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].function_invention_index, 1);
    assert_eq!(collected[0].agent_completion_index, None);
    assert_eq!(collected[0].error.code, 500);
    assert_eq!(collected[0].error.message, serde_json::Value::String("invention crashed".into()));
}

#[test]
fn inner_errors_invention_inner_only() {
    let chunk = rec_chunk(vec![
        wrapper(0, non_recursive_invention(
            vec![
                agent_completion_wrapper(0, None),
                agent_completion_wrapper(1, None),
                agent_completion_wrapper(2, Some(err(429, "rate limited"))),
            ],
            None,
        )),
    ]);
    let collected: Vec<_> = chunk.inner_errors().collect();
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].function_invention_index, 0);
    assert_eq!(collected[0].agent_completion_index, Some(2));
    assert_eq!(collected[0].error.code, 429);
}

#[test]
fn inner_errors_invention_own_and_inner_combined() {
    let chunk = rec_chunk(vec![
        wrapper(3, non_recursive_invention(
            vec![
                agent_completion_wrapper(0, Some(err(503, "unavailable"))),
            ],
            Some(err(500, "invention crashed")),
        )),
    ]);
    let collected: Vec<_> = chunk.inner_errors().collect();
    assert_eq!(collected.len(), 2);
    // own first
    assert_eq!(collected[0].function_invention_index, 3);
    assert_eq!(collected[0].agent_completion_index, None);
    assert_eq!(collected[0].error.code, 500);
    // then the inner completion error
    assert_eq!(collected[1].function_invention_index, 3);
    assert_eq!(collected[1].agent_completion_index, Some(0));
    assert_eq!(collected[1].error.code, 503);
}

#[test]
fn inner_errors_multiple_inventions() {
    let chunk = rec_chunk(vec![
        wrapper(0, non_recursive_invention(vec![], Some(err(500, "a")))),
        wrapper(2, non_recursive_invention(vec![], Some(err(502, "b")))),
    ]);
    let collected: Vec<_> = chunk.inner_errors().collect();
    assert_eq!(collected.len(), 2);
    assert_eq!(collected[0].function_invention_index, 0);
    assert_eq!(collected[0].agent_completion_index, None);
    assert_eq!(collected[0].error.code, 500);
    assert_eq!(collected[1].function_invention_index, 2);
    assert_eq!(collected[1].agent_completion_index, None);
    assert_eq!(collected[1].error.code, 502);
}

#[test]
fn inner_error_serde_roundtrip_invention_own() {
    let chunk = rec_chunk(vec![
        wrapper(7, non_recursive_invention(vec![], Some(err(404, "missing")))),
    ]);
    let item = chunk.inner_errors().next().unwrap();
    let wire = serde_json::to_string(&item).unwrap();
    assert_eq!(
        wire,
        r#"{"function_invention_index":7,"error":{"code":404,"message":"missing"}}"#,
    );
    let round: InnerError<'static> = serde_json::from_str(&wire).unwrap();
    assert_eq!(round.function_invention_index, 7);
    assert_eq!(round.agent_completion_index, None);
    assert_eq!(round.error.code, 404);
    assert_eq!(round.error.message, serde_json::Value::String("missing".into()));
}

#[test]
fn inner_error_serde_roundtrip_invention_inner() {
    let chunk = rec_chunk(vec![
        wrapper(4, non_recursive_invention(
            vec![agent_completion_wrapper(9, Some(err(418, "teapot")))],
            None,
        )),
    ]);
    let item = chunk.inner_errors().next().unwrap();
    let wire = serde_json::to_string(&item).unwrap();
    assert_eq!(
        wire,
        r#"{"function_invention_index":4,"agent_completion_index":9,"error":{"code":418,"message":"teapot"}}"#,
    );
    let round: InnerError<'static> = serde_json::from_str(&wire).unwrap();
    assert_eq!(round.function_invention_index, 4);
    assert_eq!(round.agent_completion_index, Some(9));
    assert_eq!(round.error.code, 418);
    assert_eq!(round.error.message, serde_json::Value::String("teapot".into()));
}

stream_push_test!(
    single_chunk_unchanged,
    vec![FunctionInventionRecursiveChunk {
        id: "firc-1".into(),
        inventions: vec![],
        inventions_errors: None,
        created: 100,
        object: Object::AlphaScalarFunctionInventionRecursiveChunk,
        usage: None,
    }],
    FunctionInventionRecursiveChunk {
        id: "firc-1".into(),
        inventions: vec![],
        inventions_errors: None,
        created: 100,
        object: Object::AlphaScalarFunctionInventionRecursiveChunk,
        usage: None,
    }
);

stream_push_test!(
    inventions_merged_by_index,
    vec![
        FunctionInventionRecursiveChunk {
            id: "firc-2".into(),
            inventions: vec![FunctionInventionChunk {
                index: 0,
                inner: crate::functions::inventions::response::streaming::FunctionInventionChunk {
                    id: "fi-1".into(),
                    completions: vec![],
                    state: None,
                    path: None,
                    function: None,
                    created: 100,
                    object: crate::functions::inventions::response::streaming::Object::AlphaScalarFunctionInventionChunk,
                    usage: None,
                    error: None,
                },
            }],
            inventions_errors: None,
            created: 100,
            object: Object::AlphaScalarFunctionInventionRecursiveChunk,
            usage: None,
        },
        FunctionInventionRecursiveChunk {
            id: "firc-2".into(),
            inventions: vec![FunctionInventionChunk {
                index: 1,
                inner: crate::functions::inventions::response::streaming::FunctionInventionChunk {
                    id: "fi-2".into(),
                    completions: vec![],
                    state: None,
                    path: None,
                    function: None,
                    created: 100,
                    object: crate::functions::inventions::response::streaming::Object::AlphaScalarFunctionInventionChunk,
                    usage: None,
                    error: None,
                },
            }],
            inventions_errors: None,
            created: 100,
            object: Object::AlphaScalarFunctionInventionRecursiveChunk,
            usage: None,
        },
    ],
    FunctionInventionRecursiveChunk {
        id: "firc-2".into(),
        inventions: vec![
            FunctionInventionChunk {
                index: 0,
                inner: crate::functions::inventions::response::streaming::FunctionInventionChunk {
                    id: "fi-1".into(),
                    completions: vec![],
                    state: None,
                    path: None,
                    function: None,
                    created: 100,
                    object: crate::functions::inventions::response::streaming::Object::AlphaScalarFunctionInventionChunk,
                    usage: None,
                    error: None,
                },
            },
            FunctionInventionChunk {
                index: 1,
                inner: crate::functions::inventions::response::streaming::FunctionInventionChunk {
                    id: "fi-2".into(),
                    completions: vec![],
                    state: None,
                    path: None,
                    function: None,
                    created: 100,
                    object: crate::functions::inventions::response::streaming::Object::AlphaScalarFunctionInventionChunk,
                    usage: None,
                    error: None,
                },
            },
        ],
        inventions_errors: None,
        created: 100,
        object: Object::AlphaScalarFunctionInventionRecursiveChunk,
        usage: None,
    }
);

stream_push_test!(
    inventions_errors_set,
    vec![
        FunctionInventionRecursiveChunk {
            id: "firc-3".into(),
            inventions: vec![],
            inventions_errors: None,
            created: 100,
            object: Object::AlphaVectorFunctionInventionRecursiveChunk,
            usage: None,
        },
        FunctionInventionRecursiveChunk {
            id: "firc-3".into(),
            inventions: vec![],
            inventions_errors: Some(true),
            created: 100,
            object: Object::AlphaVectorFunctionInventionRecursiveChunk,
            usage: None,
        },
    ],
    FunctionInventionRecursiveChunk {
        id: "firc-3".into(),
        inventions: vec![],
        inventions_errors: Some(true),
        created: 100,
        object: Object::AlphaVectorFunctionInventionRecursiveChunk,
        usage: None,
    }
);

stream_push_test!(
    usage_set_from_later_chunk,
    vec![
        FunctionInventionRecursiveChunk {
            id: "firc-4".into(),
            inventions: vec![],
            inventions_errors: None,
            created: 100,
            object: Object::AlphaScalarFunctionInventionRecursiveChunk,
            usage: None,
        },
        FunctionInventionRecursiveChunk {
            id: "firc-4".into(),
            inventions: vec![],
            inventions_errors: None,
            created: 100,
            object: Object::AlphaScalarFunctionInventionRecursiveChunk,
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
        },
    ],
    FunctionInventionRecursiveChunk {
        id: "firc-4".into(),
        inventions: vec![],
        inventions_errors: None,
        created: 100,
        object: Object::AlphaScalarFunctionInventionRecursiveChunk,
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
    }
);

stream_push_test!(
    usage_additive_across_chunks,
    vec![
        FunctionInventionRecursiveChunk {
            id: "firc-5".into(),
            inventions: vec![],
            inventions_errors: None,
            created: 200,
            object: Object::AlphaScalarFunctionInventionRecursiveChunk,
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
        },
        FunctionInventionRecursiveChunk {
            id: "firc-5".into(),
            inventions: vec![],
            inventions_errors: None,
            created: 200,
            object: Object::AlphaScalarFunctionInventionRecursiveChunk,
            usage: Some(crate::agent::completions::response::Usage {
                prompt_tokens: 20,
                completion_tokens: 10,
                total_tokens: 30,
                completion_tokens_details: None,
                prompt_tokens_details: None,
                cost: rust_decimal::Decimal::new(2, 3),
                cost_details: None,
                total_cost: rust_decimal::Decimal::new(2, 3),
            }),
        },
    ],
    FunctionInventionRecursiveChunk {
        id: "firc-5".into(),
        inventions: vec![],
        inventions_errors: None,
        created: 200,
        object: Object::AlphaScalarFunctionInventionRecursiveChunk,
        usage: Some(crate::agent::completions::response::Usage {
            prompt_tokens: 30,
            completion_tokens: 15,
            total_tokens: 45,
            completion_tokens_details: None,
            prompt_tokens_details: None,
            cost: rust_decimal::Decimal::new(3, 3),
            cost_details: None,
            total_cost: rust_decimal::Decimal::new(3, 3),
        }),
    }
);
