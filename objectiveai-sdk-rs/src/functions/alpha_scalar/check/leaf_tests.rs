//! Tests for check_alpha_leaf_scalar_function.

#![cfg(test)]

use crate::agent::completions::message::{RichContent, RichContentPart};
use crate::functions::alpha_scalar::{
    LeafTaskExpression, RemoteFunction, VectorCompletionTaskExpression,
};
use crate::functions::expression::{
    Expression, ImageInputSchema, InputSchema, IntegerInputSchema,
    ObjectInputSchema, StringInputSchema,
};
use crate::functions::alpha_scalar::check::check_alpha_leaf_scalar_function;
use crate::test_util::index_map;

fn test(f: &RemoteFunction) {
    check_alpha_leaf_scalar_function(f, None).unwrap();
}

fn test_err(f: &RemoteFunction, expected: &str) {
    let err = check_alpha_leaf_scalar_function(f, None).unwrap_err();
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
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "value" => InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: None,
                })
            },
            required: Some(vec!["value".to_string()]),
        },
        tasks: vec![],
    };
    test_err(&f, "AS01");
}

// --- Description checks ---

#[test]
fn description_empty() {
    let f = RemoteFunction::Leaf {
        description: "  ".to_string(),
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
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': input['text']}]}]"
                        .to_string(),
                ),
                responses: vec![
                    RichContent::Parts(vec![RichContentPart::Text {
                        text: "Option A".to_string(),
                    }]),
                    RichContent::Parts(vec![RichContentPart::Text {
                        text: "Option B".to_string(),
                    }]),
                ],
            },
        )],
    };
    test_err(&f, "QD01");
}

#[test]
fn description_too_long() {
    let f = RemoteFunction::Leaf {
        description: "a".repeat(10001),
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
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': input['text']}]}]"
                        .to_string(),
                ),
                responses: vec![
                    RichContent::Parts(vec![RichContentPart::Text {
                        text: "Option A".to_string(),
                    }]),
                    RichContent::Parts(vec![RichContentPart::Text {
                        text: "Option B".to_string(),
                    }]),
                ],
            },
        )],
    };
    test_err(&f, "QD02");
}

// --- No tasks ---

#[test]
fn rejects_no_tasks() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
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
        tasks: vec![],
    };
    test_err(&f, "AS03");
}

// --- Responses checks ---

#[test]
fn responses_less_than_2() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
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
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': input['text']}]}]"
                        .to_string(),
                ),
                responses: vec![RichContent::Parts(vec![RichContentPart::Text {
                    text: "Only one".to_string(),
                }])],
            },
        )],
    };
    test_err(&f, "AS10");
}


// --- Success cases ---

#[test]
fn valid_single_task() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
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
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': input['text']}]}]"
                        .to_string(),
                ),
                responses: vec![
                    RichContent::Parts(vec![RichContentPart::Text {
                        text: "Option A".to_string(),
                    }]),
                    RichContent::Parts(vec![RichContentPart::Text {
                        text: "Option B".to_string(),
                    }]),
                ],
            },
        )],
    };
    test(&f);
}

#[test]
fn valid_multiple_tasks() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
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
        tasks: vec![
            LeafTaskExpression::VectorCompletion(
                VectorCompletionTaskExpression {
                    skip: None,
                    messages: Expression::Starlark(
                        "[{'role': 'user', 'content': [{'type': 'text', 'text': input['text']}]}]"
                            .to_string(),
                    ),
                    responses: vec![
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "Option A".to_string(),
                        }]),
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "Option B".to_string(),
                        }]),
                    ],
                },
            ),
            LeafTaskExpression::VectorCompletion(
                VectorCompletionTaskExpression {
                    skip: None,
                    messages: Expression::Starlark(
                        "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Review: ' + input['text']}]}]"
                            .to_string(),
                    ),
                    responses: vec![
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "Good".to_string(),
                        }]),
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "Bad".to_string(),
                        }]),
                    ],
                },
            ),
            LeafTaskExpression::VectorCompletion(
                VectorCompletionTaskExpression {
                    skip: None,
                    messages: Expression::Starlark(
                        "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Rate: ' + input['text']}]}]"
                            .to_string(),
                    ),
                    responses: vec![
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "Yes".to_string(),
                        }]),
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "No".to_string(),
                        }]),
                    ],
                },
            ),
        ],
    };
    test(&f);
}

#[test]
fn valid_expression_messages() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
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
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': input['text']}]}]"
                        .to_string(),
                ),
                responses: vec![
                    RichContent::Parts(vec![RichContentPart::Text {
                        text: "option A".to_string(),
                    }]),
                    RichContent::Parts(vec![RichContentPart::Text {
                        text: "option B".to_string(),
                    }]),
                ],
            },
        )],
    };
    test(&f);
}

// --- Diversity checks ---

#[test]
fn diversity_fail_all_fixed_parameters() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
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
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'hello'}]}]"
                        .to_string(),
                ),
                responses: vec![
                    RichContent::Parts(vec![RichContentPart::Text {
                        text: "A".to_string(),
                    }]),
                    RichContent::Parts(vec![RichContentPart::Text {
                        text: "B".to_string(),
                    }]),
                ],
            },
        )],
    };
    test_err(&f, "AS19");
}

#[test]
fn diversity_fail_second_task_fixed() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
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
        tasks: vec![
            LeafTaskExpression::VectorCompletion(
                VectorCompletionTaskExpression {
                    skip: None,
                    messages: Expression::Starlark(
                        "[{'role': 'user', 'content': [{'type': 'text', 'text': input['text']}]}]"
                            .to_string(),
                    ),
                    responses: vec![
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "A".to_string(),
                        }]),
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "B".to_string(),
                        }]),
                    ],
                },
            ),
            LeafTaskExpression::VectorCompletion(
                VectorCompletionTaskExpression {
                    skip: None,
                    messages: Expression::Starlark(
                        "[{'role': 'user', 'content': [{'type': 'text', 'text': 'static prompt'}]}]"
                            .to_string(),
                    ),
                    responses: vec![
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "X".to_string(),
                        }]),
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "Y".to_string(),
                        }]),
                    ],
                },
            ),
        ],
    };
    test_err(&f, "AS19");
}

#[test]
fn diversity_fail_object_input_ignored() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "name" => InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: None,
                }),
                "score" => InputSchema::Integer(IntegerInputSchema {
                    r#type: Default::default(),
                    description: None,
                    minimum: Some(0),
                    maximum: Some(100),
                })
            },
            required: Some(vec!["name".to_string(), "score".to_string()]),
        },
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': 'rate this'}]}]"
                        .to_string(),
                ),
                responses: vec![
                    RichContent::Parts(vec![RichContentPart::Text {
                        text: "good".to_string(),
                    }]),
                    RichContent::Parts(vec![RichContentPart::Text {
                        text: "bad".to_string(),
                    }]),
                ],
            },
        )],
    };
    test_err(&f, "AS19");
}

#[test]
fn diversity_pass_message_derives_from_input() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
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
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': input['text']}]}]"
                        .to_string(),
                ),
                responses: vec![
                    RichContent::Parts(vec![RichContentPart::Text {
                        text: "yes".to_string(),
                    }]),
                    RichContent::Parts(vec![RichContentPart::Text {
                        text: "no".to_string(),
                    }]),
                ],
            },
        )],
    };
    test(&f);
}

#[test]
fn diversity_pass_object_fields_in_messages() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "question" => InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: None,
                }),
                "context" => InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: None,
                })
            },
            required: Some(vec![
                "question".to_string(),
                "context".to_string(),
            ]),
        },
        tasks: vec![
            LeafTaskExpression::VectorCompletion(
                VectorCompletionTaskExpression {
                    skip: None,
                    messages: Expression::Starlark(
                        "[{'role': 'user', 'content': [{'type': 'text', 'text': input['question']}]}]"
                            .to_string(),
                    ),
                    responses: vec![
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "yes".to_string(),
                        }]),
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "no".to_string(),
                        }]),
                    ],
                },
            ),
            LeafTaskExpression::VectorCompletion(
                VectorCompletionTaskExpression {
                    skip: None,
                    messages: Expression::Starlark(
                        "[{'role': 'user', 'content': [{'type': 'text', 'text': input['context']}]}]"
                            .to_string(),
                    ),
                    responses: vec![
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "agree".to_string(),
                        }]),
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "disagree".to_string(),
                        }]),
                    ],
                },
            ),
        ],
    };
    test(&f);
}

#[test]
fn diversity_pass_both_messages_derived() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
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
        tasks: vec![
            LeafTaskExpression::VectorCompletion(
                VectorCompletionTaskExpression {
                    skip: None,
                    messages: Expression::Starlark(
                        "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Evaluate: ' + input['text']}]}]"
                            .to_string(),
                    ),
                    responses: vec![
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "good".to_string(),
                        }]),
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "bad".to_string(),
                        }]),
                    ],
                },
            ),
            LeafTaskExpression::VectorCompletion(
                VectorCompletionTaskExpression {
                    skip: None,
                    messages: Expression::Starlark(
                        "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Rate: ' + input['text']}]}]"
                            .to_string(),
                    ),
                    responses: vec![
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "approve".to_string(),
                        }]),
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "reject".to_string(),
                        }]),
                    ],
                },
            ),
        ],
    };
    test(&f);
}

// --- Skip expression tests ---

#[test]
fn valid_with_skip_last_task_boolean() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "text" => InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: None,
                }),
                "skip_last_task" => InputSchema::Boolean(
                    crate::functions::expression::BooleanInputSchema {
                        r#type: Default::default(),
                        description: None,
                    },
                )
            },
            required: Some(vec!["text".to_string()]),
        },
        tasks: vec![
            LeafTaskExpression::VectorCompletion(
                VectorCompletionTaskExpression {
                    skip: None,
                    messages: Expression::Starlark(
                        "[{'role': 'user', 'content': [{'type': 'text', 'text': input['text']}]}]"
                            .to_string(),
                    ),
                    responses: vec![
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "Yes".to_string(),
                        }]),
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "No".to_string(),
                        }]),
                    ],
                },
            ),
            LeafTaskExpression::VectorCompletion(
                VectorCompletionTaskExpression {
                    skip: Some(Expression::Starlark(
                        "input.get('skip_last_task', False)".to_string(),
                    )),
                    messages: Expression::Starlark(
                        "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Review: ' + input['text']}]}]"
                            .to_string(),
                    ),
                    responses: vec![
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "Good".to_string(),
                        }]),
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "Bad".to_string(),
                        }]),
                    ],
                },
            ),
        ],
    };
    test(&f);
}

#[test]
fn valid_with_skip_on_high_confidence() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "text" => InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: None,
                }),
                "confidence" => InputSchema::Integer(IntegerInputSchema {
                    r#type: Default::default(),
                    description: None,
                    minimum: Some(0),
                    maximum: Some(100),
                })
            },
            required: Some(vec![
                "text".to_string(),
                "confidence".to_string(),
            ]),
        },
        tasks: vec![
            LeafTaskExpression::VectorCompletion(
                VectorCompletionTaskExpression {
                    skip: None,
                    messages: Expression::Starlark(
                        "[{'role': 'user', 'content': [{'type': 'text', 'text': input['text']}]}]"
                            .to_string(),
                    ),
                    responses: vec![
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "Agree".to_string(),
                        }]),
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "Disagree".to_string(),
                        }]),
                    ],
                },
            ),
            LeafTaskExpression::VectorCompletion(
                VectorCompletionTaskExpression {
                    skip: Some(Expression::Starlark(
                        "input['confidence'] > 75".to_string(),
                    )),
                    messages: Expression::Starlark(
                        "[{'role': 'user', 'content': [{'type': 'text', 'text': 'Confidence ' + str(input['confidence']) + ': ' + input['text']}]}]"
                            .to_string(),
                    ),
                    responses: vec![
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "Confirm".to_string(),
                        }]),
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "Reject".to_string(),
                        }]),
                    ],
                },
            ),
        ],
    };
    test(&f);
}

// --- All tasks skipped ---

#[test]
fn all_tasks_skipped() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
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
        tasks: vec![
            LeafTaskExpression::VectorCompletion(
                VectorCompletionTaskExpression {
                    skip: Some(Expression::Starlark("True".to_string())),
                    messages: Expression::Starlark(
                        "[{'role': 'user', 'content': [{'type': 'text', 'text': input['text']}]}]"
                            .to_string(),
                    ),
                    responses: vec![
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "Yes".to_string(),
                        }]),
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "No".to_string(),
                        }]),
                    ],
                },
            ),
            LeafTaskExpression::VectorCompletion(
                VectorCompletionTaskExpression {
                    skip: Some(Expression::Starlark("True".to_string())),
                    messages: Expression::Starlark(
                        "[{'role': 'user', 'content': [{'type': 'text', 'text': input['text']}]}]"
                            .to_string(),
                    ),
                    responses: vec![
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "Good".to_string(),
                        }]),
                        RichContent::Parts(vec![RichContentPart::Text {
                            text: "Bad".to_string(),
                        }]),
                    ],
                },
            ),
        ],
    };
    test_err(&f, "CV42");
}

// --- Input schema permutation checks ---

#[test]
fn rejects_single_permutation_string_enum() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "value" => InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: Some(vec!["only".to_string()]),
                })
            },
            required: Some(vec!["value".to_string()]),
        },
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': input['value']}]}]"
                        .to_string(),
                ),
                responses: vec![
                    RichContent::Parts(vec![RichContentPart::Text {
                        text: "yes".to_string(),
                    }]),
                    RichContent::Parts(vec![RichContentPart::Text {
                        text: "no".to_string(),
                    }]),
                ],
            },
        )],
    };
    test_err(&f, "QI01");
}

#[test]
fn rejects_single_permutation_integer() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "value" => InputSchema::Integer(IntegerInputSchema {
                    r#type: Default::default(),
                    description: None,
                    minimum: Some(0),
                    maximum: Some(0),
                })
            },
            required: Some(vec!["value".to_string()]),
        },
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': str(input['value'])}]}]"
                        .to_string(),
                ),
                responses: vec![
                    RichContent::Parts(vec![RichContentPart::Text {
                        text: "yes".to_string(),
                    }]),
                    RichContent::Parts(vec![RichContentPart::Text {
                        text: "no".to_string(),
                    }]),
                ],
            },
        )],
    };
    test_err(&f, "QI01");
}

// --- Multimodal coverage tests ---

#[test]
fn modality_fail_image_in_schema_but_str_in_messages() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "photo" => InputSchema::Image(ImageInputSchema {
                    r#type: Default::default(),
                    description: None,
                }),
                "label" => InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: None,
                })
            },
            required: Some(vec!["photo".to_string(), "label".to_string()]),
        },
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [{'type': 'text', 'text': input['label']}]}]"
                        .to_string(),
                ),
                responses: vec![
                    RichContent::Parts(vec![RichContentPart::Text {
                        text: "good".to_string(),
                    }]),
                    RichContent::Parts(vec![RichContentPart::Text {
                        text: "bad".to_string(),
                    }]),
                ],
            },
        )],
    };
    test_err(&f, "AS20");
}

#[test]
fn modality_pass_image_in_messages() {
    let f = RemoteFunction::Leaf {
        description: "test".to_string(),
        input_schema: ObjectInputSchema {
            r#type: Default::default(),
            description: None,
            properties: index_map! {
                "photo" => InputSchema::Image(ImageInputSchema {
                    r#type: Default::default(),
                    description: None,
                }),
                "label" => InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: None,
                })
            },
            required: Some(vec!["photo".to_string(), "label".to_string()]),
        },
        tasks: vec![LeafTaskExpression::VectorCompletion(
            VectorCompletionTaskExpression {
                skip: None,
                messages: Expression::Starlark(
                    "[{'role': 'user', 'content': [input['photo'], {'type': 'text', 'text': input['label']}]}]"
                        .to_string(),
                ),
                responses: vec![
                    RichContent::Parts(vec![RichContentPart::Text {
                        text: "good".to_string(),
                    }]),
                    RichContent::Parts(vec![RichContentPart::Text {
                        text: "bad".to_string(),
                    }]),
                ],
            },
        )],
    };
    test(&f);
}
