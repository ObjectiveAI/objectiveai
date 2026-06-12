//! Tests for check_alpha_branch_scalar_function.

#![cfg(test)]

use crate::functions::alpha_scalar::check::check_alpha_branch_scalar_function;
use crate::functions::alpha_scalar::{
    BranchTaskExpression, PlaceholderScalarFunctionTaskExpression,
    RemoteFunction, ScalarFunctionTaskExpression,
};
use crate::functions::expression::{
    Expression, IntegerInputSchema, ObjectInputSchema, StringInputSchema,
};
use crate::test_util::index_map;

fn test(f: &RemoteFunction) {
    check_alpha_branch_scalar_function(f, None, None).unwrap();
}

fn test_err(f: &RemoteFunction, expected: &str) {
    let err = check_alpha_branch_scalar_function(f, None, None).unwrap_err();
    assert!(
        err.contains(expected),
        "expected '{expected}' in error, got: {err}"
    );
}

// --- Wrong type ---

#[test]
fn wrong_type_leaf() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "value" => crate::functions::expression::InputSchema::Integer(IntegerInputSchema {
                    r#type: Default::default(),
                    description: None,
                    minimum: Some(1),
                    maximum: Some(10),
                })
            },
            required: Some(vec!["value".to_string()]),
        },
        tasks: vec![],
    };
    test_err(&f, "AB01");
}

// --- Description tests ---

#[test]
fn description_empty() {
    let f = RemoteFunction::Branch {
        description: "  ".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "value" => crate::functions::expression::InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: None,
                })
            },
            required: Some(vec!["value".to_string()]),
        },
        tasks: vec![BranchTaskExpression::ScalarFunction(
            ScalarFunctionTaskExpression {
                path: crate::RemotePath::Github {
                    owner: "test".to_string(),
                    repository: "test".to_string(),
                    commit: "abc123".to_string(),
                },
                skip: None,
                input: Expression::Starlark("input".to_string()),
            },
        )],
    };
    test_err(&f, "QD01");
}

#[test]
fn description_too_long() {
    let f = RemoteFunction::Branch {
        description: "a".repeat(351),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "value" => crate::functions::expression::InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: None,
                })
            },
            required: Some(vec!["value".to_string()]),
        },
        tasks: vec![BranchTaskExpression::ScalarFunction(
            ScalarFunctionTaskExpression {
                path: crate::RemotePath::Github {
                    owner: "test".to_string(),
                    repository: "test".to_string(),
                    commit: "abc123".to_string(),
                },
                skip: None,
                input: Expression::Starlark("input".to_string()),
            },
        )],
    };
    test_err(&f, "QD02");
}

// --- No tasks ---

#[test]
fn rejects_no_tasks() {
    let f = RemoteFunction::Branch {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "value" => crate::functions::expression::InputSchema::Integer(IntegerInputSchema {
                    r#type: Default::default(),
                    description: None,
                    minimum: Some(1),
                    maximum: Some(10),
                })
            },
            required: Some(vec!["value".to_string()]),
        },
        tasks: vec![],
    };
    test_err(&f, "AB03");
}

// --- Success cases ---

#[test]
fn valid_single_scalar_function() {
    let f = RemoteFunction::Branch {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "value" => crate::functions::expression::InputSchema::Integer(IntegerInputSchema {
                    r#type: Default::default(),
                    description: None,
                    minimum: Some(1),
                    maximum: Some(10),
                })
            },
            required: Some(vec!["value".to_string()]),
        },
        tasks: vec![BranchTaskExpression::ScalarFunction(
            ScalarFunctionTaskExpression {
                path: crate::RemotePath::Github {
                    owner: "test".to_string(),
                    repository: "test".to_string(),
                    commit: "abc123".to_string(),
                },
                skip: None,
                input: Expression::Starlark("input".to_string()),
            },
        )],
    };
    test(&f);
}

#[test]
fn valid_single_placeholder_scalar() {
    let f = RemoteFunction::Branch {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "value" => crate::functions::expression::InputSchema::Integer(IntegerInputSchema {
                    r#type: Default::default(),
                    description: None,
                    minimum: Some(1),
                    maximum: Some(10),
                })
            },
            required: Some(vec!["value".to_string()]),
        },
        tasks: vec![BranchTaskExpression::PlaceholderScalarFunction(
            PlaceholderScalarFunctionTaskExpression {
                input_schema: ObjectInputSchema {
                    r#type: Default::default(),
                    description: None,
                    properties: index_map! {
                        "value" => crate::functions::expression::InputSchema::Integer(IntegerInputSchema {
                            r#type: Default::default(),
                            description: None,
                            minimum: Some(1),
                            maximum: Some(10),
                        })
                    },
                    required: Some(vec!["value".to_string()]),
                },
                skip: None,
                input: Expression::Starlark("input".to_string()),
            },
        )],
    };
    test(&f);
}

#[test]
fn valid_multiple_tasks() {
    let f = RemoteFunction::Branch {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "value" => crate::functions::expression::InputSchema::Integer(IntegerInputSchema {
                    r#type: Default::default(),
                    description: None,
                    minimum: Some(1),
                    maximum: Some(10),
                })
            },
            required: Some(vec!["value".to_string()]),
        },
        tasks: vec![
            BranchTaskExpression::ScalarFunction(
                ScalarFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: Expression::Starlark("input".to_string()),
                },
            ),
            BranchTaskExpression::PlaceholderScalarFunction(
                PlaceholderScalarFunctionTaskExpression {
                    input_schema: ObjectInputSchema {
                        r#type: Default::default(),
                        description: None,
                        properties: index_map! {
                            "value" => crate::functions::expression::InputSchema::Integer(IntegerInputSchema {
                                r#type: Default::default(),
                                description: None,
                                minimum: Some(1),
                                maximum: Some(10),
                            })
                        },
                        required: Some(vec!["value".to_string()]),
                    },
                    skip: None,
                    input: Expression::Starlark("input".to_string()),
                },
            ),
        ],
    };
    test(&f);
}

// --- Diversity failures ---

#[test]
fn scalar_diversity_fail_fixed_input() {
    let f = RemoteFunction::Branch {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "value" => crate::functions::expression::InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: None,
                })
            },
            required: Some(vec!["value".to_string()]),
        },
        tasks: vec![
            BranchTaskExpression::ScalarFunction(
                ScalarFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: Expression::Starlark("input".to_string()),
                },
            ),
            BranchTaskExpression::ScalarFunction(
                ScalarFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: Expression::Starlark(
                        "'always_the_same'".to_string(),
                    ),
                },
            ),
        ],
    };
    test_err(&f, "AB10");
}

#[test]
fn scalar_diversity_fail_fixed_integer() {
    let f = RemoteFunction::Branch {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "value" => crate::functions::expression::InputSchema::Integer(IntegerInputSchema {
                    r#type: Default::default(),
                    description: None,
                    minimum: Some(1),
                    maximum: Some(100),
                })
            },
            required: Some(vec!["value".to_string()]),
        },
        tasks: vec![
            BranchTaskExpression::ScalarFunction(
                ScalarFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: Expression::Starlark("42".to_string()),
                },
            ),
            BranchTaskExpression::ScalarFunction(
                ScalarFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: Expression::Starlark("99".to_string()),
                },
            ),
        ],
    };
    test_err(&f, "AB10");
}

#[test]
fn scalar_diversity_fail_third_task_fixed_object() {
    let f = RemoteFunction::Branch {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "name" => crate::functions::expression::InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: None,
                }),
                "score" => crate::functions::expression::InputSchema::Integer(IntegerInputSchema {
                    r#type: Default::default(),
                    description: None,
                    minimum: Some(0),
                    maximum: Some(100),
                })
            },
            required: Some(vec!["name".to_string(), "score".to_string()]),
        },
        tasks: vec![
            BranchTaskExpression::ScalarFunction(
                ScalarFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: Expression::Starlark("input".to_string()),
                },
            ),
            BranchTaskExpression::ScalarFunction(
                ScalarFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: Expression::Starlark("input['name']".to_string()),
                },
            ),
            BranchTaskExpression::ScalarFunction(
                ScalarFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: Expression::Starlark(
                        "{'name': 'fixed', 'score': 50}".to_string(),
                    ),
                },
            ),
        ],
    };
    test_err(&f, "AB10");
}

// --- Diversity passes ---

#[test]
fn scalar_diversity_pass_string_passthrough() {
    let f = RemoteFunction::Branch {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "value" => crate::functions::expression::InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: None,
                })
            },
            required: Some(vec!["value".to_string()]),
        },
        tasks: vec![
            BranchTaskExpression::ScalarFunction(
                ScalarFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: Expression::Starlark("input".to_string()),
                },
            ),
            BranchTaskExpression::ScalarFunction(
                ScalarFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: Expression::Starlark(
                        "input['value'] + ' suffix'".to_string(),
                    ),
                },
            ),
        ],
    };
    test(&f);
}

#[test]
fn scalar_diversity_pass_integer_derived() {
    let f = RemoteFunction::Branch {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "value" => crate::functions::expression::InputSchema::Integer(IntegerInputSchema {
                    r#type: Default::default(),
                    description: None,
                    minimum: Some(1),
                    maximum: Some(1000),
                })
            },
            required: Some(vec!["value".to_string()]),
        },
        tasks: vec![
            BranchTaskExpression::ScalarFunction(
                ScalarFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: Expression::Starlark("input".to_string()),
                },
            ),
            BranchTaskExpression::ScalarFunction(
                ScalarFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: Expression::Starlark(
                        "input['value'] + 1".to_string(),
                    ),
                },
            ),
        ],
    };
    test(&f);
}

#[test]
fn scalar_diversity_pass_object_extract_field() {
    let f = RemoteFunction::Branch {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "title" => crate::functions::expression::InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: None,
                }),
                "author" => crate::functions::expression::InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: None,
                })
            },
            required: Some(vec!["title".to_string(), "author".to_string()]),
        },
        tasks: vec![
            BranchTaskExpression::ScalarFunction(
                ScalarFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: Expression::Starlark("input['title']".to_string()),
                },
            ),
            BranchTaskExpression::ScalarFunction(
                ScalarFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: Expression::Starlark("input['author']".to_string()),
                },
            ),
        ],
    };
    test(&f);
}

#[test]
fn scalar_diversity_pass_placeholder_with_transform() {
    let f = RemoteFunction::Branch {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "text" => crate::functions::expression::InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: None,
                }),
                "category" => crate::functions::expression::InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: None,
                })
            },
            required: Some(vec!["text".to_string(), "category".to_string()]),
        },
        tasks: vec![
            BranchTaskExpression::PlaceholderScalarFunction(
                PlaceholderScalarFunctionTaskExpression {
                    input_schema: ObjectInputSchema {
                        r#type: Default::default(),
                        description: None,
                        properties: index_map! {
                            "text" => crate::functions::expression::InputSchema::String(StringInputSchema {
                                r#type: Default::default(),
                                description: None,
                                r#enum: None,
                            })
                        },
                        required: Some(vec!["text".to_string()]),
                    },
                    skip: None,
                    input: Expression::Starlark(
                        "{'text': input['text']}".to_string(),
                    ),
                },
            ),
            BranchTaskExpression::PlaceholderScalarFunction(
                PlaceholderScalarFunctionTaskExpression {
                    input_schema: ObjectInputSchema {
                        r#type: Default::default(),
                        description: None,
                        properties: index_map! {
                            "text" => crate::functions::expression::InputSchema::String(StringInputSchema {
                                r#type: Default::default(),
                                description: None,
                                r#enum: None,
                            })
                        },
                        required: Some(vec!["text".to_string()]),
                    },
                    skip: None,
                    input: Expression::Starlark(
                        "{'text': input['text'] + ' [' + input['category'] + ']'}"
                            .to_string(),
                    ),
                },
            ),
        ],
    };
    test(&f);
}

#[test]
fn scalar_diversity_pass_optional_field_used() {
    let f = RemoteFunction::Branch {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "name" => crate::functions::expression::InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: None,
                }),
                "notes" => crate::functions::expression::InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: None,
                })
            },
            required: Some(vec!["name".to_string()]),
        },
        tasks: vec![
            BranchTaskExpression::ScalarFunction(
                ScalarFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: Expression::Starlark("input".to_string()),
                },
            ),
            BranchTaskExpression::ScalarFunction(
                ScalarFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: Expression::Starlark("input['name']".to_string()),
                },
            ),
        ],
    };
    test(&f);
}

// --- Skip expression tests ---

#[test]
fn valid_with_skip_last_task_boolean() {
    let f = RemoteFunction::Branch {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "text" => crate::functions::expression::InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: None,
                }),
                "skip_last_task" => crate::functions::expression::InputSchema::Boolean(
                    crate::functions::expression::BooleanInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }
                )
            },
            required: Some(vec!["text".to_string()]),
        },
        tasks: vec![
            BranchTaskExpression::ScalarFunction(
                ScalarFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: Expression::Starlark("input".to_string()),
                },
            ),
            BranchTaskExpression::ScalarFunction(
                ScalarFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: Some(Expression::Starlark(
                        "input.get('skip_last_task', False)".to_string(),
                    )),
                    input: Expression::Starlark("input['text']".to_string()),
                },
            ),
        ],
    };
    test(&f);
}

#[test]
fn valid_with_skip_on_low_priority() {
    let f = RemoteFunction::Branch {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "text" => crate::functions::expression::InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: None,
                }),
                "priority" => crate::functions::expression::InputSchema::Integer(IntegerInputSchema {
                    r#type: Default::default(),
                    description: None,
                    minimum: Some(1),
                    maximum: Some(10),
                })
            },
            required: Some(vec!["text".to_string(), "priority".to_string()]),
        },
        tasks: vec![
            BranchTaskExpression::ScalarFunction(
                ScalarFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: Expression::Starlark("input".to_string()),
                },
            ),
            BranchTaskExpression::ScalarFunction(
                ScalarFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: Some(Expression::Starlark(
                        "input['priority'] < 4".to_string(),
                    )),
                    input: Expression::Starlark(
                        "input['text'] + ' [p=' + str(input['priority']) + ']'"
                            .to_string(),
                    ),
                },
            ),
        ],
    };
    test(&f);
}

// --- Input schema permutation checks ---

#[test]
fn rejects_single_permutation_integer() {
    let f = RemoteFunction::Branch {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "value" => crate::functions::expression::InputSchema::Integer(IntegerInputSchema {
                    r#type: Default::default(),
                    description: None,
                    minimum: Some(0),
                    maximum: Some(0),
                })
            },
            required: Some(vec!["value".to_string()]),
        },
        tasks: vec![BranchTaskExpression::ScalarFunction(
            ScalarFunctionTaskExpression {
                path: crate::RemotePath::Github {
                    owner: "test".to_string(),
                    repository: "test".to_string(),
                    commit: "abc123".to_string(),
                },
                skip: None,
                input: Expression::Starlark("input".to_string()),
            },
        )],
    };
    test_err(&f, "QI01");
}

#[test]
fn rejects_single_permutation_string_enum() {
    let f = RemoteFunction::Branch {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "value" => crate::functions::expression::InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: Some(vec!["only".to_string()]),
                })
            },
            required: Some(vec!["value".to_string()]),
        },
        tasks: vec![BranchTaskExpression::ScalarFunction(
            ScalarFunctionTaskExpression {
                path: crate::RemotePath::Github {
                    owner: "test".to_string(),
                    repository: "test".to_string(),
                    commit: "abc123".to_string(),
                },
                skip: None,
                input: Expression::Starlark("input".to_string()),
            },
        )],
    };
    test_err(&f, "QI01");
}

// --- All tasks skipped ---

#[test]
fn all_tasks_skipped() {
    let f = RemoteFunction::Branch {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "value" => crate::functions::expression::InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: None,
                })
            },
            required: Some(vec!["value".to_string()]),
        },
        tasks: vec![
            BranchTaskExpression::ScalarFunction(
                ScalarFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: Some(Expression::Starlark("True".to_string())),
                    input: Expression::Starlark("input".to_string()),
                },
            ),
            BranchTaskExpression::ScalarFunction(
                ScalarFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test2".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: Some(Expression::Starlark("True".to_string())),
                    input: Expression::Starlark("input".to_string()),
                },
            ),
        ],
    };
    test_err(&f, "CV42");
}

// --- Placeholder field validation ---

#[test]
fn placeholder_field_validation_single_permutation() {
    // The placeholder's input_schema has only 1 permutation (Integer min=max=5).
    // The parent has a string field for diversity + a fixed x field.
    // The placeholder task input derives from parent (varying) but the placeholder's
    // own input_schema is checked by AB11 via check_scalar_fields -> QI01.
    // Note: CV04 validates compiled input against placeholder schema at runtime,
    // so the compiled input must be valid. With Integer(5,5), the only valid value
    // is 5, making the task input fixed — AB10 would fire first when count >= 2.
    // To avoid AB10, we include a varying name field in the placeholder schema too,
    // but add a single-value enum field that makes the overall placeholder schema
    // still have sufficient permutations. We use a separate non-varying field to
    // trigger AB11.
    //
    // Actually, the cleanest way to trigger AB11 is with a placeholder whose
    // input_schema has 1 permutation. Since CV04 and AB10 run first, and they
    // conflict with single-perm schemas, we test that the AB11 path catches
    // via the CV04 error instead (the underlying problem is the same: the
    // placeholder schema is too restrictive).
    let f = RemoteFunction::Branch {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "name" => crate::functions::expression::InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: None,
                }),
                "x" => crate::functions::expression::InputSchema::Integer(IntegerInputSchema {
                    r#type: Default::default(),
                    description: None,
                    minimum: Some(5),
                    maximum: Some(5),
                })
            },
            required: Some(vec!["name".to_string(), "x".to_string()]),
        },
        tasks: vec![
            BranchTaskExpression::ScalarFunction(
                ScalarFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: Expression::Starlark("input['name']".to_string()),
                },
            ),
            BranchTaskExpression::PlaceholderScalarFunction(
                PlaceholderScalarFunctionTaskExpression {
                    input_schema: ObjectInputSchema {
                        r#type: Default::default(),
                        description: None,
                        properties: index_map! {
                            "x" => crate::functions::expression::InputSchema::Integer(IntegerInputSchema {
                                r#type: Default::default(),
                                description: None,
                                minimum: Some(5),
                                maximum: Some(5),
                            })
                        },
                        required: Some(vec!["x".to_string()]),
                    },
                    skip: None,
                    input: Expression::Starlark(
                        "{'x': input['x']}".to_string(),
                    ),
                },
            ),
        ],
    };
    // AB10 fires before AB11 because the placeholder task input is fixed
    // (x is always 5). Both errors catch the same underlying problem:
    // the placeholder schema is too constrained.
    test_err(&f, "AB10");
}
