//! Tests for check_alpha_branch_vector_function.

#![cfg(test)]

use crate::functions::alpha_vector::check::check_alpha_branch_vector_function;
use crate::functions::alpha_vector::expression::{
    VectorFunctionInputSchema, VectorFunctionInputValueExpression,
};
use crate::functions::alpha_vector::{
    BranchTaskExpression, PlaceholderScalarFunctionTaskExpression,
    PlaceholderVectorFunctionTaskExpression, RemoteFunction,
    ScalarFunctionTaskExpression, VectorFunctionTaskExpression,
};
use crate::functions::expression::{
    BooleanInputSchema, Expression, InputSchema, ObjectInputSchema,
    StringInputSchema,
};
use crate::test_util::index_map;

fn test(f: &RemoteFunction) {
    check_alpha_branch_vector_function(f, None, None).unwrap();
}

fn test_err(f: &RemoteFunction, expected: &str) {
    let err = check_alpha_branch_vector_function(f, None, None).unwrap_err();
    assert!(
        err.contains(expected),
        "expected '{expected}' in error, got: {err}"
    );
}

// --- Structural checks ---

#[test]
fn wrong_type_leaf() {
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
    test_err(&f, "AW01");
}

#[test]
fn description_empty() {
    let f = RemoteFunction::Branch {
        description: "  ".to_string(),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: None,
            }),
        },
        tasks: vec![BranchTaskExpression::VectorFunction(
            VectorFunctionTaskExpression {
                path: crate::RemotePath::Github {
                    owner: "test".to_string(),
                    repository: "test".to_string(),
                    commit: "abc123".to_string(),
                },
                skip: None,
                input: VectorFunctionInputValueExpression {
                    context: None,
                    items: Expression::Starlark("input['items']".to_string()),
                },
            },
        )],
    };
    test_err(&f, "QD01");
}

#[test]
fn description_too_long() {
    let f = RemoteFunction::Branch {
        description: "a".repeat(351),
        input_schema: VectorFunctionInputSchema {
            context: None,
            items: InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: None,
            }),
        },
        tasks: vec![BranchTaskExpression::VectorFunction(
            VectorFunctionTaskExpression {
                path: crate::RemotePath::Github {
                    owner: "test".to_string(),
                    repository: "test".to_string(),
                    commit: "abc123".to_string(),
                },
                skip: None,
                input: VectorFunctionInputValueExpression {
                    context: None,
                    items: Expression::Starlark("input['items']".to_string()),
                },
            },
        )],
    };
    test_err(&f, "QD02");
}

#[test]
fn no_tasks() {
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
    test_err(&f, "AW02");
}

// --- Composition rules ---

#[test]
fn single_scalar_task_rejected() {
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
    test_err(&f, "AW08");
}

#[test]
fn single_placeholder_scalar_task_rejected() {
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
        tasks: vec![BranchTaskExpression::PlaceholderScalarFunction(
            PlaceholderScalarFunctionTaskExpression {
                input_schema: ObjectInputSchema {
                    r#type: Default::default(),
                    description: None,
                    properties: index_map! {
                        "text" => InputSchema::String(StringInputSchema {
                            r#type: Default::default(),
                            description: None,
                            r#enum: None,
                        })
                    },
                    required: Some(vec!["text".to_string()]),
                },
                skip: None,
                input: Expression::Starlark("input".to_string()),
            },
        )],
    };
    test_err(&f, "AW08");
}

#[test]
fn over_50_percent_scalar_tasks() {
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
                        repository: "test2".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: Expression::Starlark("input".to_string()),
                },
            ),
            BranchTaskExpression::VectorFunction(
                VectorFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: VectorFunctionInputValueExpression {
                        context: None,
                        items: Expression::Starlark(
                            "input['items']".to_string(),
                        ),
                    },
                },
            ),
        ],
    };
    test_err(&f, "AW09");
}

// --- Success cases ---

#[test]
fn valid_single_vector_function() {
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
        tasks: vec![BranchTaskExpression::VectorFunction(
            VectorFunctionTaskExpression {
                path: crate::RemotePath::Github {
                    owner: "test".to_string(),
                    repository: "test".to_string(),
                    commit: "abc123".to_string(),
                },
                skip: None,
                input: VectorFunctionInputValueExpression {
                    context: None,
                    items: Expression::Starlark("input['items']".to_string()),
                },
            },
        )],
    };
    test(&f);
}

#[test]
fn valid_single_placeholder_vector() {
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
        tasks: vec![BranchTaskExpression::PlaceholderVectorFunction(
            PlaceholderVectorFunctionTaskExpression {
                input_schema: VectorFunctionInputSchema {
                    context: None,
                    items: InputSchema::String(StringInputSchema {
                        r#type: Default::default(),
                        description: None,
                        r#enum: None,
                    }),
                },
                skip: None,
                input: VectorFunctionInputValueExpression {
                    context: None,
                    items: Expression::Starlark("input['items']".to_string()),
                },
            },
        )],
    };
    test(&f);
}

#[test]
fn valid_all_vector_tasks() {
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
        tasks: vec![
            BranchTaskExpression::VectorFunction(
                VectorFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: VectorFunctionInputValueExpression {
                        context: None,
                        items: Expression::Starlark(
                            "input['items']".to_string(),
                        ),
                    },
                },
            ),
            BranchTaskExpression::VectorFunction(
                VectorFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test2".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: VectorFunctionInputValueExpression {
                        context: None,
                        items: Expression::Starlark(
                            "input['items']".to_string(),
                        ),
                    },
                },
            ),
        ],
    };
    test(&f);
}

#[test]
fn valid_mixed_placeholder_vector_tasks() {
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
        tasks: vec![
            BranchTaskExpression::VectorFunction(
                VectorFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: VectorFunctionInputValueExpression {
                        context: None,
                        items: Expression::Starlark(
                            "input['items']".to_string(),
                        ),
                    },
                },
            ),
            BranchTaskExpression::PlaceholderVectorFunction(
                PlaceholderVectorFunctionTaskExpression {
                    input_schema: VectorFunctionInputSchema {
                        context: None,
                        items: InputSchema::String(StringInputSchema {
                            r#type: Default::default(),
                            description: None,
                            r#enum: None,
                        }),
                    },
                    skip: None,
                    input: VectorFunctionInputValueExpression {
                        context: None,
                        items: Expression::Starlark(
                            "[x + ' alt' for x in input['items']]".to_string(),
                        ),
                    },
                },
            ),
        ],
    };
    test(&f);
}

// --- Diversity checks ---

#[test]
fn input_diversity_fail_fixed_input() {
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
        tasks: vec![
            BranchTaskExpression::VectorFunction(
                VectorFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: VectorFunctionInputValueExpression {
                        context: None,
                        items: Expression::Starlark(
                            "input['items']".to_string(),
                        ),
                    },
                },
            ),
            BranchTaskExpression::VectorFunction(
                VectorFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test2".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: VectorFunctionInputValueExpression {
                        context: None,
                        items: Expression::Starlark("['A', 'B']".to_string()),
                    },
                },
            ),
        ],
    };
    test_err(&f, "AW18");
}

#[test]
fn input_diversity_pass_all_derived() {
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
        tasks: vec![
            BranchTaskExpression::VectorFunction(
                VectorFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: VectorFunctionInputValueExpression {
                        context: None,
                        items: Expression::Starlark(
                            "input['items']".to_string(),
                        ),
                    },
                },
            ),
            BranchTaskExpression::VectorFunction(
                VectorFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test2".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: VectorFunctionInputValueExpression {
                        context: None,
                        items: Expression::Starlark(
                            "input['items']".to_string(),
                        ),
                    },
                },
            ),
        ],
    };
    test(&f);
}

#[test]
fn input_diversity_pass_placeholder_vector_tasks() {
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
        tasks: vec![
            BranchTaskExpression::PlaceholderVectorFunction(
                PlaceholderVectorFunctionTaskExpression {
                    input_schema: VectorFunctionInputSchema {
                        context: None,
                        items: InputSchema::String(StringInputSchema {
                            r#type: Default::default(),
                            description: None,
                            r#enum: None,
                        }),
                    },
                    skip: None,
                    input: VectorFunctionInputValueExpression {
                        context: None,
                        items: Expression::Starlark(
                            "input['items']".to_string(),
                        ),
                    },
                },
            ),
            BranchTaskExpression::PlaceholderVectorFunction(
                PlaceholderVectorFunctionTaskExpression {
                    input_schema: VectorFunctionInputSchema {
                        context: None,
                        items: InputSchema::String(StringInputSchema {
                            r#type: Default::default(),
                            description: None,
                            r#enum: None,
                        }),
                    },
                    skip: None,
                    input: VectorFunctionInputValueExpression {
                        context: None,
                        items: Expression::Starlark(
                            "[x + ' alt' for x in input['items']]".to_string(),
                        ),
                    },
                },
            ),
        ],
    };
    test(&f);
}

// --- All tasks skipped ---

#[test]
fn all_tasks_skipped() {
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
        tasks: vec![
            BranchTaskExpression::VectorFunction(
                VectorFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: Some(Expression::Starlark("True".to_string())),
                    input: VectorFunctionInputValueExpression {
                        context: None,
                        items: Expression::Starlark(
                            "input['items']".to_string(),
                        ),
                    },
                },
            ),
            BranchTaskExpression::VectorFunction(
                VectorFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test2".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: Some(Expression::Starlark("True".to_string())),
                    input: VectorFunctionInputValueExpression {
                        context: None,
                        items: Expression::Starlark(
                            "input['items']".to_string(),
                        ),
                    },
                },
            ),
        ],
    };
    test_err(&f, "CV42");
}

// --- Skip expression tests ---
//
// Note: skip tests with context are not included because
// InputItemsOptionalContextMerge produces keys in {items, context} order
// while the transpiled schema produces {context, items} order, causing VF12
// key order mismatch. Skip tests without context work fine.

#[test]
fn valid_with_skip_on_boolean() {
    let f = RemoteFunction::Branch {
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
                    "skip_last" => InputSchema::Boolean(BooleanInputSchema {
                        r#type: Default::default(),
                        description: None,
                    })
                },
                required: Some(vec![
                    "text".to_string(),
                    "skip_last".to_string(),
                ]),
            }),
        },
        tasks: vec![
            BranchTaskExpression::VectorFunction(
                VectorFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: VectorFunctionInputValueExpression {
                        context: None,
                        items: Expression::Starlark(
                            "input['items']".to_string(),
                        ),
                    },
                },
            ),
            BranchTaskExpression::VectorFunction(
                VectorFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test2".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: Some(Expression::Starlark(
                        "input['items'][0]['skip_last']".to_string(),
                    )),
                    input: VectorFunctionInputValueExpression {
                        context: None,
                        items: Expression::Starlark(
                            "input['items']".to_string(),
                        ),
                    },
                },
            ),
        ],
    };
    test(&f);
}

#[test]
fn valid_with_skip_on_mode_enum() {
    let f = RemoteFunction::Branch {
        description: "Rank with optional deep analysis".to_string(),
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
                    "mode" => InputSchema::String(StringInputSchema {
                        r#type: Default::default(),
                        description: None,
                        r#enum: Some(vec!["quick".to_string(), "thorough".to_string()]),
                    })
                },
                required: Some(vec!["text".to_string(), "mode".to_string()]),
            }),
        },
        tasks: vec![
            BranchTaskExpression::VectorFunction(
                VectorFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: None,
                    input: VectorFunctionInputValueExpression {
                        context: None,
                        items: Expression::Starlark(
                            "input['items']".to_string(),
                        ),
                    },
                },
            ),
            BranchTaskExpression::VectorFunction(
                VectorFunctionTaskExpression {
                    path: crate::RemotePath::Github {
                        owner: "test".to_string(),
                        repository: "test2".to_string(),
                        commit: "abc123".to_string(),
                    },
                    skip: Some(Expression::Starlark(
                        "input['items'][0]['mode'] == 'quick'".to_string(),
                    )),
                    input: VectorFunctionInputValueExpression {
                        context: None,
                        items: Expression::Starlark(
                            "input['items']".to_string(),
                        ),
                    },
                },
            ),
        ],
    };
    test(&f);
}
