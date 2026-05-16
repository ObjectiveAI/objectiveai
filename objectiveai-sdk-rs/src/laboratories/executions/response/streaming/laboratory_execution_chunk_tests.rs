use crate::tests::stream_push::stream_push_test;
use super::*;

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

fn builder(index: u64, agent_index: u64, error: Option<crate::error::ResponseError>) -> BuilderChunk {
    BuilderChunk { index, agent_index, inner: agent_completion(error) }
}

fn evaluation(index: u64, agent_index: u64, error: Option<crate::error::ResponseError>) -> EvaluationChunk {
    EvaluationChunk { index, agent_index, inner: agent_completion(error), output: None }
}

fn lab_chunk_with(
    builders: Vec<BuilderChunk>,
    evaluations: Vec<EvaluationChunk>,
    error: Option<crate::error::ResponseError>,
) -> LaboratoryExecutionChunk {
    LaboratoryExecutionChunk {
        id: "lec-ie".into(),
        builders,
        evaluations,
        error,
        created: 0,
        object: Object::LaboratoryExecutionChunk,
        usage: None,
    }
}

fn err(code: u16, message: &str) -> crate::error::ResponseError {
    crate::error::ResponseError { code, message: message.into() }
}

#[test]
fn inner_errors_empty_chunk() {
    let chunk = lab_chunk_with(vec![], vec![], None);
    assert!(chunk.inner_errors().next().is_none());
}

#[test]
fn inner_errors_excludes_own_error() {
    let chunk = lab_chunk_with(vec![], vec![], Some(err(500, "lab failed")));
    assert!(chunk.inner_errors().next().is_none());
}

#[test]
fn inner_errors_only_builder_errors() {
    let chunk = lab_chunk_with(
        vec![
            builder(0, 0, None),
            builder(1, 2, Some(err(429, "rate limit"))),
        ],
        vec![],
        None,
    );
    let collected: Vec<_> = chunk.inner_errors().collect();
    assert_eq!(collected.len(), 1);
    match &collected[0] {
        InnerError::Builder { builder_index, agent_completion_index, error } => {
            assert_eq!(*builder_index, 1);
            assert_eq!(*agent_completion_index, 2);
            assert_eq!(error.code, 429);
            assert_eq!(error.message, serde_json::Value::String("rate limit".into()));
        }
        other => panic!("expected Builder, got {other:?}"),
    }
}

#[test]
fn inner_errors_only_evaluation_errors() {
    let chunk = lab_chunk_with(
        vec![],
        vec![
            evaluation(0, 1, Some(err(500, "a"))),
            evaluation(1, 0, Some(err(502, "b"))),
        ],
        None,
    );
    let collected: Vec<_> = chunk.inner_errors().collect();
    assert_eq!(collected.len(), 2);
    match &collected[0] {
        InnerError::Evaluation { evaluation_index, agent_completion_index, error } => {
            assert_eq!(*evaluation_index, 0);
            assert_eq!(*agent_completion_index, 1);
            assert_eq!(error.code, 500);
        }
        other => panic!("expected Evaluation, got {other:?}"),
    }
    match &collected[1] {
        InnerError::Evaluation { evaluation_index, agent_completion_index, error } => {
            assert_eq!(*evaluation_index, 1);
            assert_eq!(*agent_completion_index, 0);
            assert_eq!(error.code, 502);
        }
        other => panic!("expected Evaluation, got {other:?}"),
    }
}

#[test]
fn inner_errors_mixed_builders_and_evaluations() {
    let chunk = lab_chunk_with(
        vec![
            builder(0, 0, Some(err(400, "bad builder"))),
            builder(1, 0, None),
        ],
        vec![
            evaluation(0, 0, None),
            evaluation(1, 0, Some(err(503, "bad eval"))),
        ],
        None,
    );
    let collected: Vec<_> = chunk.inner_errors().collect();
    assert_eq!(collected.len(), 2);
    assert!(matches!(&collected[0], InnerError::Builder { builder_index: 0, agent_completion_index: 0, .. }));
    assert!(matches!(&collected[1], InnerError::Evaluation { evaluation_index: 1, agent_completion_index: 0, .. }));
}

#[test]
fn inner_error_serde_roundtrip_builder() {
    let chunk = lab_chunk_with(
        vec![builder(3, 4, Some(err(404, "missing")))],
        vec![],
        None,
    );
    let item = chunk.inner_errors().next().unwrap();
    let wire = serde_json::to_string(&item).unwrap();
    assert_eq!(
        wire,
        r#"{"type":"builder","builder_index":3,"agent_completion_index":4,"error":{"code":404,"message":"missing"}}"#,
    );
    let round: InnerError<'static> = serde_json::from_str(&wire).unwrap();
    match round {
        InnerError::Builder { builder_index, agent_completion_index, error } => {
            assert_eq!(builder_index, 3);
            assert_eq!(agent_completion_index, 4);
            assert_eq!(error.code, 404);
            assert_eq!(error.message, serde_json::Value::String("missing".into()));
        }
        other => panic!("expected Builder, got {other:?}"),
    }
}

#[test]
fn inner_error_serde_roundtrip_evaluation() {
    let chunk = lab_chunk_with(
        vec![],
        vec![evaluation(5, 6, Some(err(418, "teapot")))],
        None,
    );
    let item = chunk.inner_errors().next().unwrap();
    let wire = serde_json::to_string(&item).unwrap();
    assert_eq!(
        wire,
        r#"{"type":"evaluation","evaluation_index":5,"agent_completion_index":6,"error":{"code":418,"message":"teapot"}}"#,
    );
    let round: InnerError<'static> = serde_json::from_str(&wire).unwrap();
    match round {
        InnerError::Evaluation { evaluation_index, agent_completion_index, error } => {
            assert_eq!(evaluation_index, 5);
            assert_eq!(agent_completion_index, 6);
            assert_eq!(error.code, 418);
            assert_eq!(error.message, serde_json::Value::String("teapot".into()));
        }
        other => panic!("expected Evaluation, got {other:?}"),
    }
}

stream_push_test!(
    single_chunk_unchanged,
    vec![LaboratoryExecutionChunk {
        id: "lec-1".into(),
        builders: vec![],
        evaluations: vec![],
        error: None,
        created: 100,
        object: Object::LaboratoryExecutionChunk,
        usage: None,
    }],
    LaboratoryExecutionChunk {
        id: "lec-1".into(),
        builders: vec![],
        evaluations: vec![],
        error: None,
        created: 100,
        object: Object::LaboratoryExecutionChunk,
        usage: None,
    }
);

stream_push_test!(
    error_replaced_by_later_chunk,
    vec![
        LaboratoryExecutionChunk {
            id: "lec-2".into(),
            builders: vec![],
            evaluations: vec![],
            error: Some(crate::error::ResponseError {
                code: 500,
                message: serde_json::json!("first"),
            }),
            created: 100,
            object: Object::LaboratoryExecutionChunk,
            usage: None,
        },
        LaboratoryExecutionChunk {
            id: "lec-2".into(),
            builders: vec![],
            evaluations: vec![],
            error: Some(crate::error::ResponseError {
                code: 502,
                message: serde_json::json!("second"),
            }),
            created: 100,
            object: Object::LaboratoryExecutionChunk,
            usage: None,
        },
    ],
    LaboratoryExecutionChunk {
        id: "lec-2".into(),
        builders: vec![],
        evaluations: vec![],
        error: Some(crate::error::ResponseError {
            code: 502,
            message: serde_json::json!("second"),
        }),
        created: 100,
        object: Object::LaboratoryExecutionChunk,
        usage: None,
    }
);

stream_push_test!(
    usage_set_from_later_chunk,
    vec![
        LaboratoryExecutionChunk {
            id: "lec-3".into(),
            builders: vec![],
            evaluations: vec![],
            error: None,
            created: 100,
            object: Object::LaboratoryExecutionChunk,
            usage: None,
        },
        LaboratoryExecutionChunk {
            id: "lec-3".into(),
            builders: vec![],
            evaluations: vec![],
            error: None,
            created: 100,
            object: Object::LaboratoryExecutionChunk,
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
    LaboratoryExecutionChunk {
        id: "lec-3".into(),
        builders: vec![],
        evaluations: vec![],
        error: None,
        created: 100,
        object: Object::LaboratoryExecutionChunk,
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
    builders_merged_by_index,
    vec![
        LaboratoryExecutionChunk {
            id: "lec-4".into(),
            builders: vec![BuilderChunk {
                index: 0,
                agent_index: 0,
                inner: Default::default(),
            }],
            evaluations: vec![],
            error: None,
            created: 100,
            object: Object::LaboratoryExecutionChunk,
            usage: None,
        },
        LaboratoryExecutionChunk {
            id: "lec-4".into(),
            builders: vec![
                BuilderChunk {
                    index: 0,
                    agent_index: 0,
                    inner: Default::default(),
                },
                BuilderChunk {
                    index: 1,
                    agent_index: 1,
                    inner: Default::default(),
                },
            ],
            evaluations: vec![],
            error: None,
            created: 100,
            object: Object::LaboratoryExecutionChunk,
            usage: None,
        },
    ],
    LaboratoryExecutionChunk {
        id: "lec-4".into(),
        builders: vec![
            BuilderChunk {
                index: 0,
                agent_index: 0,
                inner: Default::default(),
            },
            BuilderChunk {
                index: 1,
                agent_index: 1,
                inner: Default::default(),
            },
        ],
        evaluations: vec![],
        error: None,
        created: 100,
        object: Object::LaboratoryExecutionChunk,
        usage: None,
    }
);

stream_push_test!(
    evaluations_merged_by_index,
    vec![
        LaboratoryExecutionChunk {
            id: "lec-5".into(),
            builders: vec![],
            evaluations: vec![EvaluationChunk {
                index: 0,
                agent_index: 0,
                inner: Default::default(),
                output: None,
            }],
            error: None,
            created: 100,
            object: Object::LaboratoryExecutionChunk,
            usage: None,
        },
        LaboratoryExecutionChunk {
            id: "lec-5".into(),
            builders: vec![],
            evaluations: vec![EvaluationChunk {
                index: 0,
                agent_index: 0,
                inner: Default::default(),
                output: Some(crate::functions::expression::InputValue::Integer(42)),
            }],
            error: None,
            created: 100,
            object: Object::LaboratoryExecutionChunk,
            usage: None,
        },
    ],
    LaboratoryExecutionChunk {
        id: "lec-5".into(),
        builders: vec![],
        evaluations: vec![EvaluationChunk {
            index: 0,
            agent_index: 0,
            inner: Default::default(),
            output: Some(crate::functions::expression::InputValue::Integer(42)),
        }],
        error: None,
        created: 100,
        object: Object::LaboratoryExecutionChunk,
        usage: None,
    }
);
