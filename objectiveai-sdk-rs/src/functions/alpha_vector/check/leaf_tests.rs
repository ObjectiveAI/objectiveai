//! Tests for check_alpha_leaf_vector_function.

#![cfg(test)]

use crate::functions::alpha_vector::check::check_alpha_leaf_vector_function;
use crate::functions::alpha_vector::expression::VectorFunctionInputSchema;
use crate::functions::alpha_vector::{
    LeafTaskExpression, RemoteFunction, VectorCompletionTaskExpression,
};
use crate::functions::expression::{
    ArrayInputSchema, BooleanInputSchema, Expression, ImageInputSchema,
    InputSchema, IntegerInputSchema, ObjectInputSchema, StringInputSchema,
};
use crate::test_util::index_map;

fn test(f: &RemoteFunction) {
    check_alpha_leaf_vector_function(f, None).unwrap();
}

fn test_err(f: &RemoteFunction, expected: &str) {
    let err = check_alpha_leaf_vector_function(f, None).unwrap_err();
    assert!(
        err.contains(expected),
        "expected '{expected}' in error, got: {err}"
    );
}

// --- Wrong type ---

#[test]
fn wrong_type_branch() {
    let f = RemoteFunction::Branch {
        description: "test".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: None,
            }),
        },
        tasks: vec![],
    };
    test_err(&f, "AV01");
}

// --- Description tests ---

#[test]
fn description_empty() {
    let f = RemoteFunction::Leaf {
        description: "  ".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: None,
            }),
        },
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': str(input)}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': x}] for x in input['items']]"
                        .to_string(),
                ),
            },
        )],
    };
    test_err(&f, "QD01");
}

#[test]
fn description_too_long() {
    let f = RemoteFunction::Leaf {
        description: "a".repeat(351),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: None,
            }),
        },
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': str(input)}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': x}] for x in input['items']]"
                        .to_string(),
                ),
            },
        )],
    };
    test_err(&f, "QD02");
}

// --- No tasks ---

#[test]
fn no_tasks() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: None,
            }),
        },
        tasks: vec![],
    };
    test_err(&f, "AV03");
}

// --- Success cases ---

#[test]
fn valid_simple_string_items() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: None,
            }),
        },
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'rank these'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': x}] for x in input['items']]"
                        .to_string(),
                ),
            },
        )],
    };
    test(&f);
}

#[test]
fn valid_multiple_tasks() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: None,
            }),
        },
        tasks: vec![
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'rank these'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': x}] for x in input['items']]"
                        .to_string(),
                ),
            }),
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'compare these'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': x}] for x in input['items']]"
                        .to_string(),
                ),
            }),
        ],
    };
    test(&f);
}

#[test]
fn valid_integer_items() {
    let f = RemoteFunction::Leaf {
        description: "Rank integers by preference".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::Object(ObjectInputSchema {
                r#type: Default::default(),
                description: None,
                properties: index_map! {
                    "value" => InputSchema::Integer(IntegerInputSchema {
                        r#type: Default::default(),
                        description: None,
                        minimum: Some(0),
                        maximum: Some(999),
                    }),
                    "label" => InputSchema::String(StringInputSchema {
                        r#type: Default::default(),
                        description: None,
                        r#enum: None,
                    })
                },
                required: Some(vec!["value".to_string(), "label".to_string()]),
            }),
        },
        tasks: vec![
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'rank these'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': x['label'] + ': ' + str(x['value'])}] for x in input['items']]"
                        .to_string(),
                ),
            }),
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'compare'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': 'Value #' + str(i) + ': ' + x['label']}] for i, x in enumerate(input['items'])]"
                        .to_string(),
                ),
            }),
        ],
    };
    test(&f);
}

#[test]
fn valid_object_items() {
    let f = RemoteFunction::Leaf {
        description: "Compare tagged items".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::Object(ObjectInputSchema {
                r#type: Default::default(),
                description: None,
                properties: index_map! {
                    "name" => InputSchema::String(StringInputSchema { r#type: Default::default(), description: None, r#enum: None }),
                    "tags" => InputSchema::Array(ArrayInputSchema { r#type: Default::default(), description: None, min_items: Some(1), max_items: Some(3), items: Box::new(InputSchema::String(StringInputSchema { r#type: Default::default(), description: None, r#enum: None })) })
                },
                required: Some(vec!["name".to_string(), "tags".to_string()]),
            }),
        },
        tasks: vec![
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'rank'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': x['name']}] for x in input['items']]"
                        .to_string(),
                ),
            }),
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'rank'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': x['name'] + ' [' + x['tags'][0] + ']'}] for x in input['items']]"
                        .to_string(),
                ),
            }),
        ],
    };
    test(&f);
}

#[test]
fn valid_no_max_items() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: None,
            }),
        },
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'rank'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': x}] for x in input['items']]"
                        .to_string(),
                ),
            },
        )],
    };
    test(&f);
}

// --- Response diversity (AV16 — per-index response diversity) ---

#[test]
fn responses_fixed_expression_fails_diversity() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: None,
            }),
        },
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': 'A'}], [{'type': 'text', 'text': 'B'}]]"
                        .to_string(),
                ),
            },
        )],
    };
    test_err(&f, "CV14");
}

#[test]
fn responses_fixed_pool_expression_fails_diversity() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: None,
            }),
        },
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': 'cat'}], [{'type': 'text', 'text': 'dog'}]]"
                        .to_string(),
                ),
            },
        )],
    };
    test_err(&f, "CV14");
}

#[test]
fn responses_derived_from_input_passes_diversity() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: None,
            }),
        },
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': x}] for x in input['items']]"
                        .to_string(),
                ),
            },
        )],
    };
    test(&f);
}

#[test]
fn diversity_fail_third_task_fixed_responses() {
    let f = RemoteFunction::Leaf {
        description: "Pick the better candidate".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: None,
            }),
        },
        tasks: vec![
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': c}] for c in input['items']]"
                        .to_string(),
                ),
            }),
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': 'item: ' + c}] for c in input['items']]"
                        .to_string(),
                ),
            }),
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': 'Yes'}], [{'type': 'text', 'text': 'No'}]]"
                        .to_string(),
                ),
            }),
        ],
    };
    test_err(&f, "CV14");
}

#[test]
fn diversity_fail_positional_labels() {
    let f = RemoteFunction::Leaf {
        description: "Rank entries".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: None,
            }),
        },
        tasks: vec![
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': e}] for e in input['items']]"
                        .to_string(),
                ),
            }),
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': 'entry: ' + e}] for e in input['items']]"
                        .to_string(),
                ),
            }),
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': ['First', 'Second', 'Third'][i]}] for i in range(len(input['items']))]"
                        .to_string(),
                ),
            }),
        ],
    };
    test_err(&f, "CV03");
}

// --- Passing diversity ---

#[test]
fn diversity_pass_ranking_with_enum_items() {
    let f = RemoteFunction::Leaf {
        description: "Compare options by criterion".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: None,
            }),
        },
        tasks: vec![
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': o}] for o in input['items']]"
                        .to_string(),
                ),
            }),
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': 'option: ' + o}] for o in input['items']]"
                        .to_string(),
                ),
            }),
        ],
    };
    test(&f);
}

#[test]
fn diversity_pass_multiple_tasks_with_varied_responses() {
    let f = RemoteFunction::Leaf {
        description: "Weighted choice selector".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: None,
            }),
        },
        tasks: vec![
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': c}] for c in input['items']]"
                        .to_string(),
                ),
            }),
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': 'pick: ' + c}] for c in input['items']]"
                        .to_string(),
                ),
            }),
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': 'candidate: ' + c}] for c in input['items']]"
                        .to_string(),
                ),
            }),
        ],
    };
    test(&f);
}

// --- Responses not all equal (AV17) ---

#[test]
fn within_input_responses_all_cloned() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: None,
            }),
        },
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': input['items'][0]}] for _ in input['items']]"
                        .to_string(),
                ),
            },
        )],
    };
    test_err(&f, "AV17");
}

#[test]
fn within_input_responses_cloned_two_elements() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: None,
            }),
        },
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': input['items'][0]}] for _ in input['items']]"
                        .to_string(),
                ),
            },
        )],
    };
    test_err(&f, "AV17");
}

// --- All tasks skipped (CV42) ---

#[test]
fn all_tasks_skipped() {
    let f = RemoteFunction::Leaf {
        description: "Ranks applications by quality".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: None,
            }),
        },
        tasks: vec![
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: Some(Expression::Starlark(
                    r#"len(input["items"]) < 3"#.to_string(),
                )),
                messages: Expression::Starlark(
                    r#"[{'role': 'user', 'content': [{'type': 'text', 'text': 'rank these'}]}]"#.to_string(),
                ),
                responses: Expression::Starlark(
                    r#"[[{"type": "text", "text": app}] for app in input["items"]]"#.to_string(),
                ),
            }),
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: Some(Expression::Starlark(
                    r#"len(input["items"]) < 3"#.to_string(),
                )),
                messages: Expression::Starlark(
                    r#"[{'role': 'user', 'content': [{'type': 'text', 'text': 'compare'}]}]"#.to_string(),
                ),
                responses: Expression::Starlark(
                    r#"[[{"type": "text", "text": app}] for app in input["items"]]"#.to_string(),
                ),
            }),
        ],
    };
    test_err(&f, "CV42");
}

// --- Input schema permutation checks (QI01) ---

#[test]
fn rejects_single_permutation_string_enum() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: Some(vec!["only".to_string()]),
            }),
        },
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': str(input['items'][0])}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': x}] for x in input['items']]"
                        .to_string(),
                ),
            },
        )],
    };
    test_err(&f, "AV16");
}

#[test]
fn rejects_single_permutation_integer() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::Integer(IntegerInputSchema {
                r#type: Default::default(),
                description: None,
                minimum: Some(0),
                maximum: Some(0),
            }),
        },
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': str(input['items'][0])}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': str(x)}] for x in input['items']]"
                        .to_string(),
                ),
            },
        )],
    };
    test_err(&f, "AV16");
}

// --- Multimodal coverage checks (AV18) ---

#[test]
fn modality_fail_image_in_schema_but_text_only() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::Object(ObjectInputSchema {
                r#type: Default::default(),
                description: None,
                properties: index_map! {
                    "photo" => InputSchema::Image(ImageInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                    "name" => InputSchema::String(StringInputSchema {
                        r#type: Default::default(),
                        description: None,
                        r#enum: None,
                    })
                },
                required: Some(vec!["photo".to_string(), "name".to_string()]),
            }),
        },
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'rank these'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': x['name']}] for x in input['items']]"
                        .to_string(),
                ),
            },
        )],
    };
    test_err(&f, "AV18");
}

#[test]
fn modality_pass_image_in_responses() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::Object(ObjectInputSchema {
                r#type: Default::default(),
                description: None,
                properties: index_map! {
                    "photo" => InputSchema::Image(ImageInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                    "name" => InputSchema::String(StringInputSchema {
                        r#type: Default::default(),
                        description: None,
                        r#enum: None,
                    })
                },
                required: Some(vec!["photo".to_string(), "name".to_string()]),
            }),
        },
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'rank these photos'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[x['photo']] for x in input['items']]".to_string(),
                ),
            },
        )],
    };
    test(&f);
}

#[test]
fn modality_pass_image_in_messages() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::Object(ObjectInputSchema {
                r#type: Default::default(),
                description: None,
                properties: index_map! {
                    "photo" => InputSchema::Image(ImageInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                    "name" => InputSchema::String(StringInputSchema {
                        r#type: Default::default(),
                        description: None,
                        r#enum: None,
                    })
                },
                required: Some(vec!["photo".to_string(), "name".to_string()]),
            }),
        },
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [x['photo'] for x in input['items']]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': x['name']}] for x in input['items']]"
                        .to_string(),
                ),
            },
        )],
    };
    test(&f);
}

// --- Merged sub-input validation ---

#[test]
fn valid_merged_subsets_simple() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: None,
            }),
        },
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'rank'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': x}] for x in input['items']]"
                        .to_string(),
                ),
            },
        )],
    };
    test(&f);
}

// --- Skip expression tests ---

#[test]
fn valid_with_skip_last_task_boolean() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::Object(ObjectInputSchema {
                r#type: Default::default(),
                description: None,
                properties: index_map! {
                    "text" => InputSchema::String(StringInputSchema {
                        r#type: Default::default(),
                        description: None,
                        r#enum: None,
                    }),
                    "skip_last_task" => InputSchema::Boolean(BooleanInputSchema {
                        r#type: Default::default(),
                        description: None,
                    })
                },
                required: Some(vec!["text".to_string(), "skip_last_task".to_string()]),
            }),
        },
        tasks: vec![
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': x['text']}] for x in input['items']]"
                        .to_string(),
                ),
            }),
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: Some(Expression::Starlark(
                    "input['items'][0]['skip_last_task']".to_string(),
                )),
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': 'alt: ' + x['text']}] for x in input['items']]"
                        .to_string(),
                ),
            }),
        ],
    };
    test(&f);
}

#[test]
fn valid_with_skip_on_quick_mode() {
    let f = RemoteFunction::Leaf {
        description: "Rank with optional deep analysis".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: None,
            }),
        },
        tasks: vec![
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': x}] for x in input['items']]"
                        .to_string(),
                ),
            }),
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: Some(Expression::Starlark(
                    "len(input['items']) > 5".to_string(),
                )),
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': 'deep: ' + x}] for x in input['items']]"
                        .to_string(),
                ),
            }),
        ],
    };
    test(&f);
}

// --- People ranking with optional fields ---

#[test]
fn valid_people_ranking_with_skip() {
    let f = RemoteFunction::Leaf {
        description: "Rank people by name quality".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::Object(ObjectInputSchema {
                r#type: Default::default(),
                description: None,
                properties: index_map! {
                    "fullName" => InputSchema::String(StringInputSchema { r#type: Default::default(), description: None, r#enum: None }),
                    "firstName" => InputSchema::String(StringInputSchema { r#type: Default::default(), description: None, r#enum: None }),
                    "lastName" => InputSchema::String(StringInputSchema { r#type: Default::default(), description: None, r#enum: None })
                },
                required: Some(vec!["fullName".to_string(), "firstName".to_string()]),
            }),
        },
        tasks: vec![
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': p['fullName']}] for p in input['items']]"
                        .to_string(),
                ),
            }),
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': p['firstName']}] for p in input['items']]"
                        .to_string(),
                ),
            }),
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: Some(Expression::Starlark(
                    "any([p.get('lastName') == None for p in input['items']])".to_string(),
                )),
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': p['lastName']}] for p in input['items']]"
                        .to_string(),
                ),
            }),
        ],
    };
    test(&f);
}

#[test]
fn people_ranking_null_lastname_replaced() {
    let f = RemoteFunction::Leaf {
        description: "Rank people by name quality".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::Object(ObjectInputSchema {
                r#type: Default::default(),
                description: None,
                properties: index_map! {
                    "fullName" => InputSchema::String(StringInputSchema { r#type: Default::default(), description: None, r#enum: None }),
                    "firstName" => InputSchema::String(StringInputSchema { r#type: Default::default(), description: None, r#enum: None }),
                    "lastName" => InputSchema::String(StringInputSchema { r#type: Default::default(), description: None, r#enum: None })
                },
                required: Some(vec!["fullName".to_string(), "firstName".to_string()]),
            }),
        },
        tasks: vec![
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': p['fullName']}] for p in input['items']]"
                        .to_string(),
                ),
            }),
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': p['firstName']}] for p in input['items']]"
                        .to_string(),
                ),
            }),
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': p.get('lastName', 'NULL')}] for p in input['items']]"
                        .to_string(),
                ),
            }),
        ],
    };
    test(&f);
}

// --- Response diversity with optional-field items ---

#[test]
fn response_diversity_pass_boolean_derived() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::Object(ObjectInputSchema {
                r#type: Default::default(),
                description: None,
                properties: index_map! {
                    "label" => InputSchema::String(StringInputSchema { r#type: Default::default(), description: None, r#enum: None }),
                    "flag" => InputSchema::Boolean(BooleanInputSchema { r#type: Default::default(), description: None })
                },
                required: None,
            }),
        },
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': str(x.get('flag', 'NULL'))}] for x in input['items']]"
                        .to_string(),
                ),
            },
        )],
    };
    test(&f);
}

#[test]
fn response_diversity_fail_fixed_responses() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::Object(ObjectInputSchema {
                r#type: Default::default(),
                description: None,
                properties: index_map! {
                    "label" => InputSchema::String(StringInputSchema { r#type: Default::default(), description: None, r#enum: None }),
                    "flag" => InputSchema::Boolean(BooleanInputSchema { r#type: Default::default(), description: None })
                },
                required: None,
            }),
        },
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}]"
                        .to_string(),
                ),
                responses: Expression::Starlark(
                    "[[{'type': 'text', 'text': str(i)}] for i in range(len(input['items']))]"
                        .to_string(),
                ),
            },
        )],
    };
    test_err(&f, "AV16");
}

// --- Job application ranker (passing) ---

#[test]
fn job_application_ranker_pass() {
    let f = RemoteFunction::Leaf {
        description: "Ranks job applications against a job description by evaluating multiple dimensions".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: None,
            }),
        },
        tasks: vec![
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    r#"[{'role': 'user', 'content': [{'type': 'text', 'text': "Skills alignment"}]}]"#.to_string(),
                ),
                responses: Expression::Starlark(
                    r#"[[{"type": "text", "text": app}] for app in input["items"]]"#.to_string(),
                ),
            }),
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    r#"[{'role': 'user', 'content': [{'type': 'text', 'text': "Impact assessment"}]}]"#.to_string(),
                ),
                responses: Expression::Starlark(
                    r#"[[{"type": "text", "text": app}] for app in input["items"]]"#.to_string(),
                ),
            }),
            LeafTaskExpression::VectorCompletion(VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    r#"[{'role': 'user', 'content': [{'type': 'text', 'text': "Communication clarity"}]}]"#.to_string(),
                ),
                responses: Expression::Starlark(
                    r#"[[{"type": "text", "text": app}] for app in input["items"]]"#.to_string(),
                ),
            }),
        ],
    };
    test(&f);
}
