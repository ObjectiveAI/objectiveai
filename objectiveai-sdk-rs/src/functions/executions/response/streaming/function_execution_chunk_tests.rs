use crate::tests::stream_push::stream_push_test;
use super::*;

fn err(code: u16, message: &str) -> crate::error::ResponseError {
    crate::error::ResponseError { code, message: message.into() }
}

fn agent_completion(error: Option<crate::error::ResponseError>) -> crate::agent::completions::response::streaming::AgentCompletionChunk {
    crate::agent::completions::response::streaming::AgentCompletionChunk {
        id: String::new(),
        created: 0,
        messages: vec![],
        object: crate::agent::completions::response::streaming::Object::AgentCompletionChunk,
        usage: None,
        upstream: crate::agent::Upstream::Openrouter,
        error,
        continuation: None,
    }
}

fn vector_agent_completion(
    index: u64,
    error: Option<crate::error::ResponseError>,
) -> crate::vector::completions::response::streaming::AgentCompletionChunk {
    crate::vector::completions::response::streaming::AgentCompletionChunk {
        index,
        inner: agent_completion(error),
    }
}

fn vector_completion_chunk(
    completions: Vec<crate::vector::completions::response::streaming::AgentCompletionChunk>,
) -> crate::vector::completions::response::streaming::VectorCompletionChunk {
    crate::vector::completions::response::streaming::VectorCompletionChunk {
        id: String::new(),
        completions,
        votes: vec![],
        scores: vec![],
        weights: vec![],
        created: 0,
        swarm: String::new(),
        object: crate::vector::completions::response::streaming::Object::VectorCompletionChunk,
        usage: None,
    }
}

fn vct(
    task_path: Vec<u64>,
    own_error: Option<crate::error::ResponseError>,
    completions: Vec<crate::vector::completions::response::streaming::AgentCompletionChunk>,
) -> VectorCompletionTaskChunk {
    VectorCompletionTaskChunk {
        index: *task_path.last().unwrap_or(&0),
        task_index: *task_path.last().unwrap_or(&0),
        task_path,
        inner: vector_completion_chunk(completions),
        error: own_error,
    }
}

fn fet(
    task_path: Vec<u64>,
    swiss_pool_index: Option<u64>,
    swiss_round: Option<u64>,
    split_index: Option<u64>,
    inner: FunctionExecutionChunk,
) -> FunctionExecutionTaskChunk {
    FunctionExecutionTaskChunk {
        index: *task_path.last().unwrap_or(&0),
        task_index: *task_path.last().unwrap_or(&0),
        task_path,
        swiss_pool_index,
        swiss_round,
        split_index,
        inner,
    }
}

fn reasoning(
    own_error: Option<crate::error::ResponseError>,
    agent_error: Option<crate::error::ResponseError>,
) -> ReasoningSummaryChunk {
    ReasoningSummaryChunk {
        inner: agent_completion(agent_error),
        error: own_error,
    }
}

fn chunk(
    tasks: Vec<TaskChunk>,
    reasoning: Option<ReasoningSummaryChunk>,
    own_error: Option<crate::error::ResponseError>,
) -> FunctionExecutionChunk {
    FunctionExecutionChunk {
        id: "fec-ie".into(),
        tasks,
        tasks_errors: None,
        reasoning,
        output: None,
        error: own_error,
        retry_token: None,
        created: 0,
        function: None,
        profile: None,
        object: Object::ScalarFunctionExecutionChunk,
        usage: None,
    }
}

#[test]
fn inner_errors_empty() {
    let c = chunk(vec![], None, None);
    assert!(c.inner_errors().next().is_none());
}

#[test]
fn inner_errors_excludes_own_error() {
    let c = chunk(vec![], None, Some(err(500, "execution failed")));
    assert!(c.inner_errors().next().is_none());
}

#[test]
fn inner_errors_function_task_error() {
    let inner = chunk(vec![], None, Some(err(503, "nested failed")));
    let task = TaskChunk::FunctionExecution(fet(
        vec![0],
        Some(2),
        Some(1),
        None,
        inner,
    ));
    let c = chunk(vec![task], None, None);
    let collected: Vec<_> = c.inner_errors().collect();
    assert_eq!(collected.len(), 1);
    match &collected[0] {
        InnerError::FunctionTaskError {
            task_path, swiss_pool_index, swiss_round, split_index, error,
        } => {
            assert_eq!(task_path, &vec![0]);
            assert_eq!(*swiss_pool_index, Some(2));
            assert_eq!(*swiss_round, Some(1));
            assert_eq!(*split_index, None);
            assert_eq!(error.code, 503);
        }
        other => panic!("expected FunctionTaskError, got {other:?}"),
    }
}

#[test]
fn inner_errors_vector_completion_task_own_error() {
    let task = TaskChunk::VectorCompletion(vct(
        vec![1],
        Some(err(429, "rate limit")),
        vec![],
    ));
    let c = chunk(vec![task], None, None);
    let collected: Vec<_> = c.inner_errors().collect();
    assert_eq!(collected.len(), 1);
    match &collected[0] {
        InnerError::VectorCompletionTaskError { task_path, agent_completion_index, error } => {
            assert_eq!(task_path, &vec![1]);
            assert_eq!(*agent_completion_index, None);
            assert_eq!(error.code, 429);
        }
        other => panic!("expected VectorCompletionTaskError, got {other:?}"),
    }
}

#[test]
fn inner_errors_vector_completion_agent_errors() {
    let task = TaskChunk::VectorCompletion(vct(
        vec![0],
        None,
        vec![
            vector_agent_completion(0, Some(err(500, "a"))),
            vector_agent_completion(1, None),
            vector_agent_completion(2, Some(err(502, "c"))),
        ],
    ));
    let c = chunk(vec![task], None, None);
    let collected: Vec<_> = c.inner_errors().collect();
    assert_eq!(collected.len(), 2);
    match &collected[0] {
        InnerError::VectorCompletionTaskError { agent_completion_index, error, .. } => {
            assert_eq!(*agent_completion_index, Some(0));
            assert_eq!(error.code, 500);
        }
        other => panic!("expected VectorCompletionTaskError, got {other:?}"),
    }
    match &collected[1] {
        InnerError::VectorCompletionTaskError { agent_completion_index, error, .. } => {
            assert_eq!(*agent_completion_index, Some(2));
            assert_eq!(error.code, 502);
        }
        other => panic!("expected VectorCompletionTaskError, got {other:?}"),
    }
}

#[test]
fn inner_errors_skips_reasoning_summary_own_error() {
    let c = chunk(vec![], Some(reasoning(Some(err(500, "summary failed")), None)), None);
    assert!(c.inner_errors().next().is_none());
}

#[test]
fn inner_errors_reasoning_agent_completion_error() {
    let c = chunk(vec![], Some(reasoning(None, Some(err(500, "agent failed")))), None);
    let collected: Vec<_> = c.inner_errors().collect();
    assert_eq!(collected.len(), 1);
    match &collected[0] {
        InnerError::ReasoningAgentCompletionError { task_path, error } => {
            assert!(task_path.is_empty());
            assert_eq!(error.code, 500);
        }
        other => panic!("expected ReasoningAgentCompletionError, got {other:?}"),
    }
}

#[test]
fn inner_errors_recursion_two_levels_deep() {
    let inner_vector_task = TaskChunk::VectorCompletion(vct(
        vec![0, 0],
        Some(err(400, "inner task own")),
        vec![
            vector_agent_completion(0, None),
            vector_agent_completion(1, Some(err(503, "inner completion 1"))),
        ],
    ));
    let nested_exec = chunk(vec![inner_vector_task], None, None);
    let outer_task = TaskChunk::FunctionExecution(fet(vec![0], None, None, None, nested_exec));
    let c = chunk(vec![outer_task], None, None);

    let collected: Vec<_> = c.inner_errors().collect();
    assert_eq!(collected.len(), 2);
    // No FunctionTaskError because the wrapper's inner.error is None.
    match &collected[0] {
        InnerError::VectorCompletionTaskError { task_path, agent_completion_index, error } => {
            assert_eq!(task_path, &vec![0, 0]);
            assert_eq!(*agent_completion_index, None);
            assert_eq!(error.code, 400);
        }
        other => panic!("expected VectorCompletionTaskError, got {other:?}"),
    }
    match &collected[1] {
        InnerError::VectorCompletionTaskError { task_path, agent_completion_index, error } => {
            assert_eq!(task_path, &vec![0, 0]);
            assert_eq!(*agent_completion_index, Some(1));
            assert_eq!(error.code, 503);
        }
        other => panic!("expected VectorCompletionTaskError, got {other:?}"),
    }
}

#[test]
fn inner_errors_recursion_nested_reasoning() {
    let nested_exec = chunk(
        vec![],
        Some(reasoning(None, Some(err(500, "nested reasoning agent")))),
        None,
    );
    let outer_task = TaskChunk::FunctionExecution(fet(vec![3], None, None, None, nested_exec));
    let c = chunk(vec![outer_task], None, None);

    let collected: Vec<_> = c.inner_errors().collect();
    assert_eq!(collected.len(), 1);
    match &collected[0] {
        InnerError::ReasoningAgentCompletionError { task_path, error } => {
            assert_eq!(task_path, &vec![3]);
            assert_eq!(error.code, 500);
        }
        other => panic!("expected ReasoningAgentCompletionError, got {other:?}"),
    }
}

#[test]
fn inner_errors_mixed_kitchen_sink() {
    // Root contains:
    //   tasks[0]: function-execution wrapper at [0], inner.error set; recurses; nested has no children.
    //   tasks[1]: vector-completion wrapper at [1], own error + one agent error at completion idx 4.
    //   reasoning: agent inner.error set.
    let nested_exec = chunk(vec![], None, Some(err(503, "deep nested"))); // its .error is set
    let t0 = TaskChunk::FunctionExecution(fet(vec![0], None, None, None, nested_exec));
    let t1 = TaskChunk::VectorCompletion(vct(
        vec![1],
        Some(err(429, "task1 own")),
        vec![
            vector_agent_completion(0, None),
            vector_agent_completion(4, Some(err(500, "t1 c4"))),
        ],
    ));
    let c = chunk(
        vec![t0, t1],
        Some(reasoning(None, Some(err(599, "reasoning agent")))),
        None,
    );

    let collected: Vec<_> = c.inner_errors().collect();
    assert_eq!(collected.len(), 4);
    // Order: t0 FunctionTaskError, t1 VectorCompletionTaskError(None), t1 VectorCompletionTaskError(Some(4)), reasoning agent
    assert!(matches!(&collected[0], InnerError::FunctionTaskError { task_path, .. } if task_path == &vec![0]));
    assert!(matches!(&collected[1], InnerError::VectorCompletionTaskError { task_path, agent_completion_index: None, .. } if task_path == &vec![1]));
    assert!(matches!(&collected[2], InnerError::VectorCompletionTaskError { task_path, agent_completion_index: Some(4), .. } if task_path == &vec![1]));
    assert!(matches!(&collected[3], InnerError::ReasoningAgentCompletionError { task_path, .. } if task_path.is_empty()));
}

#[test]
fn inner_error_serde_roundtrip_function_task_error() {
    let item = InnerError::FunctionTaskError {
        task_path: vec![0, 1],
        swiss_pool_index: Some(2),
        swiss_round: Some(3),
        split_index: None,
        error: std::borrow::Cow::Owned(err(404, "missing")),
    };
    let wire = serde_json::to_string(&item).unwrap();
    assert_eq!(
        wire,
        r#"{"type":"function_task_error","task_path":[0,1],"swiss_pool_index":2,"swiss_round":3,"error":{"code":404,"message":"missing"}}"#,
    );
    let round: InnerError<'static> = serde_json::from_str(&wire).unwrap();
    assert_eq!(round, item);
}

#[test]
fn inner_error_serde_roundtrip_vector_completion_task_error_own() {
    let item = InnerError::VectorCompletionTaskError {
        task_path: vec![2],
        agent_completion_index: None,
        error: std::borrow::Cow::Owned(err(429, "rate")),
    };
    let wire = serde_json::to_string(&item).unwrap();
    assert_eq!(
        wire,
        r#"{"type":"vector_completion_task_error","task_path":[2],"error":{"code":429,"message":"rate"}}"#,
    );
    let round: InnerError<'static> = serde_json::from_str(&wire).unwrap();
    assert_eq!(round, item);
}

#[test]
fn inner_error_serde_roundtrip_vector_completion_task_error_agent() {
    let item = InnerError::VectorCompletionTaskError {
        task_path: vec![0, 0],
        agent_completion_index: Some(7),
        error: std::borrow::Cow::Owned(err(503, "down")),
    };
    let wire = serde_json::to_string(&item).unwrap();
    assert_eq!(
        wire,
        r#"{"type":"vector_completion_task_error","task_path":[0,0],"agent_completion_index":7,"error":{"code":503,"message":"down"}}"#,
    );
    let round: InnerError<'static> = serde_json::from_str(&wire).unwrap();
    assert_eq!(round, item);
}

#[test]
fn inner_error_serde_roundtrip_reasoning_agent_completion_error() {
    let item = InnerError::ReasoningAgentCompletionError {
        task_path: vec![5],
        error: std::borrow::Cow::Owned(err(418, "teapot")),
    };
    let wire = serde_json::to_string(&item).unwrap();
    assert_eq!(
        wire,
        r#"{"type":"reasoning_agent_completion_error","task_path":[5],"error":{"code":418,"message":"teapot"}}"#,
    );
    let round: InnerError<'static> = serde_json::from_str(&wire).unwrap();
    assert_eq!(round, item);
}

stream_push_test!(
    single_chunk_unchanged,
    vec![FunctionExecutionChunk {
        id: "fec-1".into(),
        tasks: vec![],
        tasks_errors: None,
        reasoning: None,
        output: None,
        error: None,
        retry_token: None,
        created: 100,
        function: None,
        profile: None,
        object: Object::ScalarFunctionExecutionChunk,
        usage: None,
    }],
    FunctionExecutionChunk {
        id: "fec-1".into(),
        tasks: vec![],
        tasks_errors: None,
        reasoning: None,
        output: None,
        error: None,
        retry_token: None,
        created: 100,
        function: None,
        profile: None,
        object: Object::ScalarFunctionExecutionChunk,
        usage: None,
    }
);

stream_push_test!(
    output_set_from_later_chunk,
    vec![
        FunctionExecutionChunk {
            id: "fec-2".into(),
            tasks: vec![],
            tasks_errors: None,
            reasoning: None,
            output: None,
            error: None,
            retry_token: None,
            created: 100,
            function: None,
            profile: None,
            object: Object::ScalarFunctionExecutionChunk,
            usage: None,
        },
        FunctionExecutionChunk {
            id: "fec-2".into(),
            tasks: vec![],
            tasks_errors: None,
            reasoning: None,
            output: Some(super::super::Output { output: crate::functions::expression::TaskOutputOwned::Scalar(
                rust_decimal::Decimal::new(75, 2),
            ) }),
            error: None,
            retry_token: None,
            created: 100,
            function: None,
            profile: None,
            object: Object::ScalarFunctionExecutionChunk,
            usage: None,
        },
    ],
    FunctionExecutionChunk {
        id: "fec-2".into(),
        tasks: vec![],
        tasks_errors: None,
        reasoning: None,
        output: Some(super::super::Output { output: crate::functions::expression::TaskOutputOwned::Scalar(
            rust_decimal::Decimal::new(75, 2),
        ) }),
        error: None,
        retry_token: None,
        created: 100,
        function: None,
        profile: None,
        object: Object::ScalarFunctionExecutionChunk,
        usage: None,
    }
);

stream_push_test!(
    output_replaced_by_later_chunk,
    vec![
        FunctionExecutionChunk {
            id: "fec-3".into(),
            tasks: vec![],
            tasks_errors: None,
            reasoning: None,
            output: Some(super::super::Output { output: crate::functions::expression::TaskOutputOwned::Scalar(
                rust_decimal::Decimal::new(25, 2),
            ) }),
            error: None,
            retry_token: None,
            created: 100,
            function: None,
            profile: None,
            object: Object::ScalarFunctionExecutionChunk,
            usage: None,
        },
        FunctionExecutionChunk {
            id: "fec-3".into(),
            tasks: vec![],
            tasks_errors: None,
            reasoning: None,
            output: Some(super::super::Output { output: crate::functions::expression::TaskOutputOwned::Scalar(
                rust_decimal::Decimal::new(75, 2),
            ) }),
            error: None,
            retry_token: None,
            created: 100,
            function: None,
            profile: None,
            object: Object::ScalarFunctionExecutionChunk,
            usage: None,
        },
    ],
    FunctionExecutionChunk {
        id: "fec-3".into(),
        tasks: vec![],
        tasks_errors: None,
        reasoning: None,
        output: Some(super::super::Output { output: crate::functions::expression::TaskOutputOwned::Scalar(
            rust_decimal::Decimal::new(75, 2),
        ) }),
        error: None,
        retry_token: None,
        created: 100,
        function: None,
        profile: None,
        object: Object::ScalarFunctionExecutionChunk,
        usage: None,
    }
);

stream_push_test!(
    usage_set_from_later_chunk,
    vec![
        FunctionExecutionChunk {
            id: "fec-4".into(),
            tasks: vec![],
            tasks_errors: None,
            reasoning: None,
            output: None,
            error: None,
            retry_token: None,
            created: 100,
            function: None,
            profile: None,
            object: Object::ScalarFunctionExecutionChunk,
            usage: None,
        },
        FunctionExecutionChunk {
            id: "fec-4".into(),
            tasks: vec![],
            tasks_errors: None,
            reasoning: None,
            output: None,
            error: None,
            retry_token: None,
            created: 100,
            function: None,
            profile: None,
            object: Object::ScalarFunctionExecutionChunk,
            usage: Some(crate::agent::completions::response::Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
                completion_tokens_details: None,
                prompt_tokens_details: None,
                cost: rust_decimal::Decimal::ZERO,
                cost_details: None,
                total_cost: rust_decimal::Decimal::ZERO,
            }),
        },
    ],
    FunctionExecutionChunk {
        id: "fec-4".into(),
        tasks: vec![],
        tasks_errors: None,
        reasoning: None,
        output: None,
        error: None,
        retry_token: None,
        created: 100,
        function: None,
        profile: None,
        object: Object::ScalarFunctionExecutionChunk,
        usage: Some(crate::agent::completions::response::Usage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
            completion_tokens_details: None,
            prompt_tokens_details: None,
            cost: rust_decimal::Decimal::ZERO,
            cost_details: None,
            total_cost: rust_decimal::Decimal::ZERO,
        }),
    }
);

stream_push_test!(
    error_replaced_by_later_chunk,
    vec![
        FunctionExecutionChunk {
            id: "fec-5".into(),
            tasks: vec![],
            tasks_errors: None,
            reasoning: None,
            output: None,
            error: Some(crate::error::ResponseError {
                code: 500,
                message: serde_json::json!("first"),
            }),
            retry_token: None,
            created: 100,
            function: None,
            profile: None,
            object: Object::ScalarFunctionExecutionChunk,
            usage: None,
        },
        FunctionExecutionChunk {
            id: "fec-5".into(),
            tasks: vec![],
            tasks_errors: None,
            reasoning: None,
            output: None,
            error: Some(crate::error::ResponseError {
                code: 502,
                message: serde_json::json!("second"),
            }),
            retry_token: None,
            created: 100,
            function: None,
            profile: None,
            object: Object::ScalarFunctionExecutionChunk,
            usage: None,
        },
    ],
    FunctionExecutionChunk {
        id: "fec-5".into(),
        tasks: vec![],
        tasks_errors: None,
        reasoning: None,
        output: None,
        error: Some(crate::error::ResponseError {
            code: 502,
            message: serde_json::json!("second"),
        }),
        retry_token: None,
        created: 100,
        function: None,
        profile: None,
        object: Object::ScalarFunctionExecutionChunk,
        usage: None,
    }
);
