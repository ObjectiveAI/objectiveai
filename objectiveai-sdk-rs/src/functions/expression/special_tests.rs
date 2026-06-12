use super::*;
use crate::functions::expression::{
    ExpressionError, InputValue, InputValueExpression, Params, ParamsOwned,
    TaskOutputOwned,
};
use indexmap::IndexMap;
use rust_decimal::dec;

// ── Special::Input ──────────────────────────────────────────────────

#[test]
fn special_input_returns_string_input() {
    let params = Params::Owned(ParamsOwned {
        input: InputValue::String("hello".to_string()),
        output: None,
        map: None,
    });
    let result = InputValue::from_special(&Special::Input, &params).unwrap();
    assert_eq!(result, InputValue::String("hello".to_string()));
}

#[test]
fn special_input_returns_object_input() {
    let mut obj = IndexMap::new();
    obj.insert("name".to_string(), InputValue::String("alice".to_string()));
    obj.insert("age".to_string(), InputValue::Integer(30));
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Object(obj.clone()),
        output: None,
        map: None,
    });
    let result = InputValue::from_special(&Special::Input, &params).unwrap();
    assert_eq!(result, InputValue::Object(obj));
}

#[test]
fn special_input_returns_input_expression() {
    let params = Params::Owned(ParamsOwned {
        input: InputValue::String("hello".to_string()),
        output: None,
        map: None,
    });
    let result =
        InputValueExpression::from_special(&Special::Input, &params).unwrap();
    assert!(matches!(result, InputValueExpression::String(s) if s == "hello"));
}

#[test]
fn special_input_returns_object_input_expression() {
    let mut obj = IndexMap::new();
    obj.insert("x".to_string(), InputValue::Integer(42));
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Object(obj),
        output: None,
        map: None,
    });
    let result =
        InputValueExpression::from_special(&Special::Input, &params).unwrap();
    match result {
        InputValueExpression::Object(map) => {
            assert!(map.contains_key("x"));
        }
        other => {
            panic!("expected InputValueExpression::Object, got {:?}", other)
        }
    }
}

// ── Special::Input failures ─────────────────────────────────────────

#[test]
fn special_input_fails_for_bool() {
    let params = Params::Owned(ParamsOwned {
        input: InputValue::String("hello".to_string()),
        output: None,
        map: None,
    });
    let result = bool::from_special(&Special::Input, &params);
    assert!(matches!(result, Err(ExpressionError::UnsupportedSpecial)));
}

#[test]
fn special_input_fails_for_task_output() {
    let params = Params::Owned(ParamsOwned {
        input: InputValue::String("hello".to_string()),
        output: None,
        map: None,
    });
    let result = TaskOutputOwned::from_special(&Special::Input, &params);
    assert!(matches!(result, Err(ExpressionError::UnsupportedSpecial)));
}

// ── Special::Output ─────────────────────────────────────────────────

#[test]
fn special_output_returns_scalar() {
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Boolean(true),
        output: Some(TaskOutputOwned::Scalar(dec!(0.75))),
        map: None,
    });
    let result =
        TaskOutputOwned::from_special(&Special::Output, &params).unwrap();
    assert!(matches!(result, TaskOutputOwned::Scalar(d) if d == dec!(0.75)));
}

#[test]
fn special_output_returns_vector() {
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Boolean(true),
        output: Some(TaskOutputOwned::Vector(vec![dec!(0.3), dec!(0.7)])),
        map: None,
    });
    let result =
        TaskOutputOwned::from_special(&Special::Output, &params).unwrap();
    match result {
        TaskOutputOwned::Vector(v) => {
            assert_eq!(v, vec![dec!(0.3), dec!(0.7)])
        }
        other => panic!("expected Vector, got {:?}", other),
    }
}

// ── Special::Output failures ────────────────────────────────────────

#[test]
fn special_output_fails_for_input() {
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Boolean(true),
        output: Some(TaskOutputOwned::Scalar(dec!(0.5))),
        map: None,
    });
    let result = InputValue::from_special(&Special::Output, &params);
    assert!(matches!(result, Err(ExpressionError::UnsupportedSpecial)));
}

#[test]
fn special_output_fails_for_u64() {
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Boolean(true),
        output: Some(TaskOutputOwned::Scalar(dec!(0.5))),
        map: None,
    });
    let result = u64::from_special(&Special::Output, &params);
    assert!(matches!(result, Err(ExpressionError::UnsupportedSpecial)));
}

// ── Special::TaskOutputL1Normalized ──────────────────────────────

#[test]
fn special_l1_norm_normalizes_vector() {
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Boolean(true),
        output: Some(TaskOutputOwned::Vector(vec![dec!(2), dec!(3), dec!(5)])),
        map: None,
    });
    let result = TaskOutputOwned::from_special(
        &Special::TaskOutputL1Normalized,
        &params,
    )
    .unwrap();
    match result {
        TaskOutputOwned::Vector(v) => {
            assert_eq!(v, vec![dec!(0.2), dec!(0.3), dec!(0.5)]);
        }
        other => panic!("expected Vector, got {:?}", other),
    }
}

#[test]
fn special_l1_norm_normalizes_mapped_scalars_as_vector() {
    // Mapped scalars are now represented as Vector
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Boolean(true),
        output: Some(TaskOutputOwned::Vector(vec![dec!(1), dec!(3)])),
        map: None,
    });
    let result = TaskOutputOwned::from_special(
        &Special::TaskOutputL1Normalized,
        &params,
    )
    .unwrap();
    match result {
        TaskOutputOwned::Vector(v) => {
            assert_eq!(v, vec![dec!(0.25), dec!(0.75)]);
        }
        other => panic!("expected Vector, got {:?}", other),
    }
}

// ── Special::TaskOutputL1Normalized failures ─────────────────────

#[test]
fn special_l1_norm_fails_for_input() {
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Boolean(true),
        output: Some(TaskOutputOwned::Vector(vec![dec!(1)])),
        map: None,
    });
    let result =
        InputValue::from_special(&Special::TaskOutputL1Normalized, &params);
    assert!(matches!(result, Err(ExpressionError::UnsupportedSpecial)));
}

#[test]
fn special_l1_norm_fails_for_string() {
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Boolean(true),
        output: Some(TaskOutputOwned::Vector(vec![dec!(1)])),
        map: None,
    });
    let result =
        String::from_special(&Special::TaskOutputL1Normalized, &params);
    assert!(matches!(result, Err(ExpressionError::UnsupportedSpecial)));
}

// ── Special::InputItemsOutputLength ──────────────────────────────────

#[test]
fn special_items_output_length_returns_count() {
    let mut obj = IndexMap::new();
    obj.insert(
        "items".to_string(),
        InputValue::Array(vec![
            InputValue::String("a".to_string()),
            InputValue::String("b".to_string()),
            InputValue::String("c".to_string()),
        ]),
    );
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Object(obj),
        output: None,
        map: None,
    });
    let result =
        u64::from_special(&Special::InputItemsOutputLength, &params).unwrap();
    assert_eq!(result, 3);
}

#[test]
fn special_items_output_length_returns_zero_for_empty() {
    let mut obj = IndexMap::new();
    obj.insert("items".to_string(), InputValue::Array(vec![]));
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Object(obj),
        output: None,
        map: None,
    });
    let result =
        u64::from_special(&Special::InputItemsOutputLength, &params).unwrap();
    assert_eq!(result, 0);
}

// ── Special::InputItemsOutputLength failures ────────────────────────

#[test]
fn special_items_output_length_fails_for_non_object_input() {
    let params = Params::Owned(ParamsOwned {
        input: InputValue::String("hello".to_string()),
        output: None,
        map: None,
    });
    let result = u64::from_special(&Special::InputItemsOutputLength, &params);
    assert!(matches!(result, Err(ExpressionError::UnsupportedSpecial)));
}

#[test]
fn special_items_output_length_fails_for_missing_items() {
    let mut obj = IndexMap::new();
    obj.insert("name".to_string(), InputValue::String("alice".to_string()));
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Object(obj),
        output: None,
        map: None,
    });
    let result = u64::from_special(&Special::InputItemsOutputLength, &params);
    assert!(matches!(result, Err(ExpressionError::UnsupportedSpecial)));
}

// ── Special::InputItemsOptionalContextSplit ─────────────────────────

#[test]
fn special_split_with_context() {
    let mut obj = IndexMap::new();
    obj.insert(
        "items".to_string(),
        InputValue::Array(vec![
            InputValue::String("x".to_string()),
            InputValue::String("y".to_string()),
        ]),
    );
    obj.insert("context".to_string(), InputValue::String("ctx".to_string()));
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Object(obj),
        output: None,
        map: None,
    });
    let result = Vec::<InputValue>::from_special(
        &Special::InputItemsOptionalContextSplit,
        &params,
    )
    .unwrap();
    assert_eq!(result.len(), 2);
    // First element: items=["x"], context="ctx"
    match &result[0] {
        InputValue::Object(m) => {
            assert_eq!(
                m.get("items").unwrap(),
                &InputValue::Array(vec![InputValue::String("x".to_string())])
            );
            assert_eq!(
                m.get("context").unwrap(),
                &InputValue::String("ctx".to_string())
            );
        }
        other => panic!("expected Object, got {:?}", other),
    }
    // Second element: items=["y"], context="ctx"
    match &result[1] {
        InputValue::Object(m) => {
            assert_eq!(
                m.get("items").unwrap(),
                &InputValue::Array(vec![InputValue::String("y".to_string())])
            );
            assert_eq!(
                m.get("context").unwrap(),
                &InputValue::String("ctx".to_string())
            );
        }
        other => panic!("expected Object, got {:?}", other),
    }
}

#[test]
fn special_split_without_context() {
    let mut obj = IndexMap::new();
    obj.insert(
        "items".to_string(),
        InputValue::Array(vec![
            InputValue::Integer(1),
            InputValue::Integer(2),
            InputValue::Integer(3),
        ]),
    );
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Object(obj),
        output: None,
        map: None,
    });
    let result = Vec::<InputValue>::from_special(
        &Special::InputItemsOptionalContextSplit,
        &params,
    )
    .unwrap();
    assert_eq!(result.len(), 3);
    match &result[0] {
        InputValue::Object(m) => {
            assert_eq!(
                m.get("items").unwrap(),
                &InputValue::Array(vec![InputValue::Integer(1)])
            );
            assert!(m.get("context").is_none());
        }
        other => panic!("expected Object, got {:?}", other),
    }
    match &result[2] {
        InputValue::Object(m) => {
            assert_eq!(
                m.get("items").unwrap(),
                &InputValue::Array(vec![InputValue::Integer(3)])
            );
        }
        other => panic!("expected Object, got {:?}", other),
    }
}

// ── Special::InputItemsOptionalContextSplit failures ─────────────────

#[test]
fn special_split_fails_for_non_object_input() {
    let params = Params::Owned(ParamsOwned {
        input: InputValue::String("hello".to_string()),
        output: None,
        map: None,
    });
    let result = Vec::<InputValue>::from_special(
        &Special::InputItemsOptionalContextSplit,
        &params,
    );
    assert!(matches!(result, Err(ExpressionError::UnsupportedSpecial)));
}

#[test]
fn special_split_fails_for_missing_items() {
    let mut obj = IndexMap::new();
    obj.insert("name".to_string(), InputValue::String("alice".to_string()));
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Object(obj),
        output: None,
        map: None,
    });
    let result = Vec::<InputValue>::from_special(
        &Special::InputItemsOptionalContextSplit,
        &params,
    );
    assert!(matches!(result, Err(ExpressionError::UnsupportedSpecial)));
}

// ── Special::InputItemsOptionalContextMerge ─────────────────────────

#[test]
fn special_merge_with_context() {
    let sub1 = {
        let mut m = IndexMap::new();
        m.insert(
            "items".to_string(),
            InputValue::Array(vec![InputValue::String("a".to_string())]),
        );
        m.insert("context".to_string(), InputValue::String("ctx".to_string()));
        InputValue::Object(m)
    };
    let sub2 = {
        let mut m = IndexMap::new();
        m.insert(
            "items".to_string(),
            InputValue::Array(vec![InputValue::String("b".to_string())]),
        );
        m.insert("context".to_string(), InputValue::String("ctx".to_string()));
        InputValue::Object(m)
    };
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Array(vec![sub1, sub2]),
        output: None,
        map: None,
    });
    let result = InputValue::from_special(
        &Special::InputItemsOptionalContextMerge,
        &params,
    )
    .unwrap();
    match result {
        InputValue::Object(m) => {
            assert_eq!(
                m.get("items").unwrap(),
                &InputValue::Array(vec![
                    InputValue::String("a".to_string()),
                    InputValue::String("b".to_string()),
                ])
            );
            assert_eq!(
                m.get("context").unwrap(),
                &InputValue::String("ctx".to_string())
            );
        }
        other => panic!("expected Object, got {:?}", other),
    }
}

#[test]
fn special_merge_without_context() {
    let sub1 = {
        let mut m = IndexMap::new();
        m.insert(
            "items".to_string(),
            InputValue::Array(vec![InputValue::Integer(10)]),
        );
        InputValue::Object(m)
    };
    let sub2 = {
        let mut m = IndexMap::new();
        m.insert(
            "items".to_string(),
            InputValue::Array(vec![InputValue::Integer(20)]),
        );
        InputValue::Object(m)
    };
    let sub3 = {
        let mut m = IndexMap::new();
        m.insert(
            "items".to_string(),
            InputValue::Array(vec![InputValue::Integer(30)]),
        );
        InputValue::Object(m)
    };
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Array(vec![sub1, sub2, sub3]),
        output: None,
        map: None,
    });
    let result = InputValue::from_special(
        &Special::InputItemsOptionalContextMerge,
        &params,
    )
    .unwrap();
    match result {
        InputValue::Object(m) => {
            assert_eq!(
                m.get("items").unwrap(),
                &InputValue::Array(vec![
                    InputValue::Integer(10),
                    InputValue::Integer(20),
                    InputValue::Integer(30)
                ])
            );
            assert!(m.get("context").is_none());
        }
        other => panic!("expected Object, got {:?}", other),
    }
}

// ── Special::InputItemsOptionalContextMerge failures ─────────────────

#[test]
fn special_merge_fails_for_non_array_input() {
    let params = Params::Owned(ParamsOwned {
        input: InputValue::String("hello".to_string()),
        output: None,
        map: None,
    });
    let result = InputValue::from_special(
        &Special::InputItemsOptionalContextMerge,
        &params,
    );
    assert!(matches!(result, Err(ExpressionError::UnsupportedSpecial)));
}

#[test]
fn special_merge_fails_for_non_object_elements() {
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Array(vec![
            InputValue::Integer(1),
            InputValue::Integer(2),
        ]),
        output: None,
        map: None,
    });
    let result = InputValue::from_special(
        &Special::InputItemsOptionalContextMerge,
        &params,
    );
    assert!(matches!(result, Err(ExpressionError::UnsupportedSpecial)));
}

// ── Special::Output (was VectorCompletionScores) ──────────────────────

#[test]
fn special_output_returns_scores_vector() {
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Boolean(true),
        output: Some(TaskOutputOwned::Vector(vec![dec!(0.6), dec!(0.4)])),
        map: None,
    });
    let result =
        TaskOutputOwned::from_special(&Special::Output, &params).unwrap();
    match result {
        TaskOutputOwned::Vector(v) => {
            assert_eq!(v, vec![dec!(0.6), dec!(0.4)])
        }
        other => panic!("expected Vector, got {:?}", other),
    }
}

#[test]
fn special_output_returns_three_scores() {
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Boolean(true),
        output: Some(TaskOutputOwned::Vector(vec![
            dec!(0.2),
            dec!(0.3),
            dec!(0.5),
        ])),
        map: None,
    });
    let result =
        TaskOutputOwned::from_special(&Special::Output, &params).unwrap();
    match result {
        TaskOutputOwned::Vector(v) => {
            assert_eq!(v, vec![dec!(0.2), dec!(0.3), dec!(0.5)])
        }
        other => panic!("expected Vector, got {:?}", other),
    }
}

// ── Special::Output failures ────────────────────────────────────────

#[test]
fn special_output_fails_for_input_type() {
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Boolean(true),
        output: Some(TaskOutputOwned::Vector(vec![dec!(0.5), dec!(0.5)])),
        map: None,
    });
    let result = InputValue::from_special(&Special::Output, &params);
    assert!(matches!(result, Err(ExpressionError::UnsupportedSpecial)));
}

#[test]
fn special_output_fails_for_no_output() {
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Boolean(true),
        output: None,
        map: None,
    });
    let result = TaskOutputOwned::from_special(&Special::Output, &params);
    assert!(matches!(result, Err(ExpressionError::UnsupportedSpecial)));
}

// ── Special::TaskOutputWeightedSum ───────────────────────────────────

#[test]
fn special_weighted_sum_two_scores() {
    // scores=[0.6, 0.4], weights=[0/1, 1/1]=[0, 1]
    // weighted_sum = 0.6*0 + 0.4*1 = 0.4
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Boolean(true),
        output: Some(TaskOutputOwned::Vector(vec![dec!(0.6), dec!(0.4)])),
        map: None,
    });
    let result =
        TaskOutputOwned::from_special(&Special::TaskOutputWeightedSum, &params)
            .unwrap();
    assert!(matches!(result, TaskOutputOwned::Scalar(d) if d == dec!(0.4)));
}

#[test]
fn special_weighted_sum_three_scores() {
    // scores=[0.2, 0.3, 0.5], weights=[0/2, 1/2, 2/2]=[0, 0.5, 1]
    // weighted_sum = 0.2*0 + 0.3*0.5 + 0.5*1 = 0 + 0.15 + 0.5 = 0.65
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Boolean(true),
        output: Some(TaskOutputOwned::Vector(vec![
            dec!(0.2),
            dec!(0.3),
            dec!(0.5),
        ])),
        map: None,
    });
    let result =
        TaskOutputOwned::from_special(&Special::TaskOutputWeightedSum, &params)
            .unwrap();
    assert!(matches!(result, TaskOutputOwned::Scalar(d) if d == dec!(0.65)));
}

// ── Special::TaskOutputWeightedSum failures ──────────────────────────

#[test]
fn special_weighted_sum_fails_for_input() {
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Boolean(true),
        output: Some(TaskOutputOwned::Vector(vec![dec!(0.5), dec!(0.5)])),
        map: None,
    });
    let result =
        InputValue::from_special(&Special::TaskOutputWeightedSum, &params);
    assert!(matches!(result, Err(ExpressionError::UnsupportedSpecial)));
}

#[test]
fn special_weighted_sum_fails_for_no_output() {
    let params = Params::Owned(ParamsOwned {
        input: InputValue::Boolean(true),
        output: None,
        map: None,
    });
    let result =
        TaskOutputOwned::from_special(&Special::TaskOutputWeightedSum, &params);
    assert!(matches!(result, Err(ExpressionError::UnsupportedSpecial)));
}
