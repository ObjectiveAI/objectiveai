use super::starlark::with_eval_result;
use super::*;
use crate::agent::completions::message::{
    AssistantMessageExpression, AssistantToolCallExpression,
    AssistantToolCallFunctionExpression, File, ImageUrl, ImageUrlDetail,
    InputAudio, MessageExpression, RichContentExpression,
    RichContentPartExpression, ToolMessageExpression, UserMessageExpression,
    VideoUrl,
};
use crate::functions::expression::{
    InputValue, InputValueExpression, Params, ParamsOwned, TaskOutputOwned,
    WithExpression,
};
use indexmap::IndexMap;
use rust_decimal::dec;
use serde::Serialize;

/// Evaluate a Starlark expression and convert the result to `T`.
fn starlark_eval<T: FromStarlarkValue>(
    code: &str,
    params: &super::Params,
) -> Result<T, ExpressionError> {
    with_eval_result(code, params, T::from_starlark_value)
}

fn assert_starlark_deep_eq<T: FromStarlarkValue + Serialize>(
    code: &str,
    params: &Params<'_, '_>,
    expected: &T,
) {
    let result: T = starlark_eval(code, params).unwrap();
    assert_eq!(
        serde_json::to_value(&result).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
}

fn empty_input() -> InputValue {
    InputValue::Object(IndexMap::new())
}

fn make_params(input: InputValue) -> Params<'static, 'static> {
    Params::Owned(ParamsOwned {
        input,
        output: None,
        map: None,
    })
}

fn make_params_with_output(
    input: InputValue,
    output: TaskOutputOwned,
) -> Params<'static, 'static> {
    Params::Owned(ParamsOwned {
        input,
        output: Some(output),
        map: None,
    })
}

fn make_params_with_map(
    input: InputValue,
    map: u64,
) -> Params<'static, 'static> {
    Params::Owned(ParamsOwned {
        input,
        output: None,
        map: Some(map),
    })
}

fn make_full_params(
    input: InputValue,
    output: TaskOutputOwned,
    map: u64,
) -> Params<'static, 'static> {
    Params::Owned(ParamsOwned {
        input,
        output: Some(output),
        map: Some(map),
    })
}

// Helper to build an object InputValue
fn obj(pairs: Vec<(&str, InputValue)>) -> InputValue {
    InputValue::Object(
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
    )
}

// Helper to build an array InputValue
fn arr(items: Vec<InputValue>) -> InputValue {
    InputValue::Array(items)
}

#[test]
fn test_simple_literal() {
    let params = make_params(empty_input());
    let result: i64 = starlark_eval("42", &params).unwrap();
    assert_eq!(result, 42);
}

#[test]
fn test_string_literal() {
    let params = make_params(empty_input());
    let result: String = starlark_eval("\"hello\"", &params).unwrap();
    assert_eq!(result, "hello");
}

#[test]
fn test_boolean_literal() {
    let params = make_params(empty_input());
    assert_eq!(starlark_eval::<bool>("True", &params).unwrap(), true);
    assert_eq!(starlark_eval::<bool>("False", &params).unwrap(), false);
}

#[test]
fn test_none_literal() {
    let params = make_params(empty_input());
    let result: Option<i64> = starlark_eval("None", &params).unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_list_literal() {
    let params = make_params(empty_input());
    let result: Vec<i64> = starlark_eval("[1, 2, 3]", &params).unwrap();
    assert_eq!(result, vec![1, 2, 3]);
}

#[test]
fn test_input_access() {
    let input = obj(vec![
        ("name", InputValue::String("alice".to_string())),
        ("age", InputValue::Integer(30)),
    ]);
    let params = make_params(input);
    let name: String = starlark_eval("input['name']", &params).unwrap();
    assert_eq!(name, "alice");
    let age: i64 = starlark_eval("input['age']", &params).unwrap();
    assert_eq!(age, 30);
}

#[test]
fn test_nested_input_access() {
    let input = obj(vec![(
        "user",
        obj(vec![(
            "profile",
            obj(vec![(
                "email",
                InputValue::String("test@example.com".to_string()),
            )]),
        )]),
    )]);
    let params = make_params(input);
    let result: String =
        starlark_eval("input['user']['profile']['email']", &params).unwrap();
    assert_eq!(result, "test@example.com");
}

#[test]
fn test_array_indexing() {
    let input = obj(vec![(
        "items",
        arr(vec![
            InputValue::String("a".to_string()),
            InputValue::String("b".to_string()),
            InputValue::String("c".to_string()),
        ]),
    )]);
    let params = make_params(input);
    let v0: String = starlark_eval("input['items'][0]", &params).unwrap();
    assert_eq!(v0, "a");
    let v1: String = starlark_eval("input['items'][-1]", &params).unwrap();
    assert_eq!(v1, "c");
}

#[test]
fn test_list_comprehension() {
    let input = obj(vec![(
        "numbers",
        arr(vec![
            InputValue::Integer(1),
            InputValue::Integer(2),
            InputValue::Integer(3),
        ]),
    )]);
    let params = make_params(input);
    let result: Vec<i64> =
        starlark_eval("[x * 2 for x in input['numbers']]", &params).unwrap();
    assert_eq!(result, vec![2, 4, 6]);
}

#[test]
fn test_list_comprehension_with_filter() {
    let input = obj(vec![(
        "numbers",
        arr(vec![
            InputValue::Integer(1),
            InputValue::Integer(2),
            InputValue::Integer(3),
            InputValue::Integer(4),
            InputValue::Integer(5),
            InputValue::Integer(6),
        ]),
    )]);
    let params = make_params(input);
    let result: Vec<i64> =
        starlark_eval("[x for x in input['numbers'] if x % 2 == 0]", &params)
            .unwrap();
    assert_eq!(result, vec![2, 4, 6]);
}

#[test]
fn test_arithmetic() {
    let params = make_params(empty_input());
    assert_eq!(starlark_eval::<i64>("1 + 2", &params).unwrap(), 3);
    assert_eq!(starlark_eval::<i64>("10 - 3", &params).unwrap(), 7);
    assert_eq!(starlark_eval::<i64>("4 * 5", &params).unwrap(), 20);
    assert_eq!(starlark_eval::<i64>("15 // 4", &params).unwrap(), 3);
    assert_eq!(starlark_eval::<i64>("15 % 4", &params).unwrap(), 3);
}

#[test]
fn test_builtin_functions() {
    let params = make_params(empty_input());
    assert_eq!(starlark_eval::<i64>("min([3, 1, 2])", &params).unwrap(), 1);
    assert_eq!(starlark_eval::<i64>("max([3, 1, 2])", &params).unwrap(), 3);
    assert_eq!(starlark_eval::<i64>("len([1, 2, 3])", &params).unwrap(), 3);
    let sorted: Vec<i64> = starlark_eval("sorted([3, 1, 2])", &params).unwrap();
    assert_eq!(sorted, vec![1, 2, 3]);
}

#[test]
fn test_conditional_expression() {
    let input = obj(vec![("value", InputValue::Integer(10))]);
    let params = make_params(input);
    let result: String =
        starlark_eval("\"big\" if input['value'] > 5 else \"small\"", &params)
            .unwrap();
    assert_eq!(result, "big");

    let input2 = obj(vec![("value", InputValue::Integer(3))]);
    let params2 = make_params(input2);
    let result2: String =
        starlark_eval("\"big\" if input['value'] > 5 else \"small\"", &params2)
            .unwrap();
    assert_eq!(result2, "small");
}

#[test]
fn test_parse_error() {
    let params = make_params(empty_input());
    let result: Result<i64, _> = starlark_eval("invalid syntax [[[", &params);
    assert!(matches!(
        result,
        Err(ExpressionError::StarlarkParseError(_))
    ));
}

#[test]
fn test_eval_error() {
    let params = make_params(empty_input());
    let result: Result<i64, _> = starlark_eval("undefined_variable", &params);
    assert!(matches!(result, Err(ExpressionError::StarlarkEvalError(_))));
}

// ==================== TESTS USING MAP ====================

#[test]
fn test_map_access() {
    let input = obj(vec![
        ("base", InputValue::Integer(100)),
        (
            "multipliers",
            arr(vec![InputValue::Integer(3), InputValue::Integer(5)]),
        ),
    ]);
    let params = make_params_with_map(input, 0);

    let result: i64 =
        starlark_eval("input['base'] * input['multipliers'][map]", &params)
            .unwrap();
    assert_eq!(result, 300);
}

#[test]
fn test_map_as_index() {
    let input = obj(vec![(
        "items",
        arr(vec![
            InputValue::String("Hello".to_string()),
            InputValue::String("World".to_string()),
        ]),
    )]);
    let params = make_params_with_map(input, 1);

    let result: String = starlark_eval("input['items'][map]", &params).unwrap();
    assert_eq!(result, "World");

    let result2: i64 = starlark_eval("map * 10", &params).unwrap();
    assert_eq!(result2, 10);
}

#[test]
fn test_map_in_list_comprehension() {
    let input = obj(vec![
        ("factor", InputValue::Integer(2)),
        (
            "values",
            arr(vec![
                InputValue::Integer(10),
                InputValue::Integer(20),
                InputValue::Integer(30),
            ]),
        ),
    ]);
    let params = make_params_with_map(input, 1);

    let result: i64 =
        starlark_eval("input['values'][map] * input['factor']", &params)
            .unwrap();
    assert_eq!(result, 40);
}

// ==================== TESTS USING OUTPUT ====================

#[test]
fn test_output_scalar() {
    let input = empty_input();
    let output = TaskOutputOwned::Scalar(dec!(0.75));
    let params = make_params_with_output(input, output);

    let result: f64 = starlark_eval("output", &params).unwrap();
    assert!((result - 0.75).abs() < 1e-9);
}

#[test]
fn test_output_vector() {
    let input = empty_input();
    let output = TaskOutputOwned::Vector(vec![dec!(0.1), dec!(0.2), dec!(0.7)]);
    let params = make_params_with_output(input, output);

    let result: i64 = starlark_eval("len(output)", &params).unwrap();
    assert_eq!(result, 3);
}

#[test]
fn test_output_vector_scores() {
    let input = empty_input();
    let output =
        TaskOutputOwned::Vector(vec![dec!(0.25), dec!(0.25), dec!(0.5)]);
    let params = make_params_with_output(input, output);

    let result: i64 = starlark_eval("len(output)", &params).unwrap();
    assert_eq!(result, 3);
}

#[test]
fn test_output_none() {
    let input = empty_input();
    let params = make_params(input);

    let result: bool = starlark_eval("output == None", &params).unwrap();
    assert!(result);
}

#[test]
fn test_output_not_none() {
    let input = obj(vec![("threshold", InputValue::Number(0.5))]);
    let output = TaskOutputOwned::Scalar(dec!(0.7));
    let params = make_params_with_output(input, output);

    let result: bool = starlark_eval("output != None", &params).unwrap();
    assert!(result);
}

// ==================== TESTS USING ALL THREE ====================

#[test]
fn test_full_params_all_fields() {
    let input = obj(vec![("base_score", InputValue::Number(0.5))]);
    let output = TaskOutputOwned::Vector(vec![dec!(0.3), dec!(0.7)]);
    let params = make_full_params(input, output, 1);

    let result: i64 = starlark_eval("len(output)", &params).unwrap();
    assert_eq!(result, 2);

    let result2: i64 = starlark_eval("map", &params).unwrap();
    assert_eq!(result2, 1);
}

#[test]
fn test_full_params_complex_expression() {
    let input = obj(vec![(
        "items",
        arr(vec![
            InputValue::String("a".to_string()),
            InputValue::String("b".to_string()),
            InputValue::String("c".to_string()),
        ]),
    )]);
    let output = TaskOutputOwned::Scalar(dec!(0.5));
    let params = make_full_params(input, output, 1);

    let result: String = starlark_eval("input['items'][map]", &params).unwrap();
    assert_eq!(result, "b");

    let result2: f64 = starlark_eval("output", &params).unwrap();
    assert!((result2 - 0.5).abs() < 1e-9);
}

#[test]
fn test_map_function_outputs() {
    let input = empty_input();
    let output = TaskOutputOwned::Vector(vec![dec!(0.1), dec!(0.5), dec!(0.9)]);
    let params = make_params_with_output(input, output);

    let result: i64 = starlark_eval("len(output)", &params).unwrap();
    assert_eq!(result, 3);
}

#[test]
fn test_map_vector_outputs() {
    let input = empty_input();
    let output = TaskOutputOwned::Vectors(vec![
        vec![dec!(0.5), dec!(0.5)],
        vec![dec!(0.3), dec!(0.7)],
    ]);
    let params = make_params_with_output(input, output);

    let result: i64 = starlark_eval("len(output[0])", &params).unwrap();
    assert_eq!(result, 2);

    let result2: i64 = starlark_eval("len(output[1])", &params).unwrap();
    assert_eq!(result2, 2);
}

// ==================== TESTS FOR CUSTOM FUNCTIONS ====================

#[test]
fn test_sum_integers() {
    let params = make_params(empty_input());
    let result: f64 = starlark_eval("sum([1, 2, 3, 4, 5])", &params).unwrap();
    assert_eq!(result, 15.0);
}

#[test]
fn test_sum_floats() {
    let params = make_params(empty_input());
    let result: f64 = starlark_eval("sum([1.5, 2.5, 3.0])", &params).unwrap();
    assert_eq!(result, 7.0);
}

#[test]
fn test_sum_empty() {
    let params = make_params(empty_input());
    let result: f64 = starlark_eval("sum([])", &params).unwrap();
    assert_eq!(result, 0.0);
}

#[test]
fn test_sum_from_input() {
    let input = obj(vec![(
        "values",
        arr(vec![
            InputValue::Integer(10),
            InputValue::Integer(20),
            InputValue::Integer(30),
        ]),
    )]);
    let params = make_params(input);
    let result: f64 = starlark_eval("sum(input['values'])", &params).unwrap();
    assert_eq!(result, 60.0);
}

#[test]
fn test_abs_positive() {
    let params = make_params(empty_input());
    let result: f64 = starlark_eval("abs(5)", &params).unwrap();
    assert_eq!(result, 5.0);
}

#[test]
fn test_abs_negative() {
    let params = make_params(empty_input());
    let result: f64 = starlark_eval("abs(-5)", &params).unwrap();
    assert_eq!(result, 5.0);
}

#[test]
fn test_abs_float() {
    let params = make_params(empty_input());
    let result: f64 = starlark_eval("abs(-3.14)", &params).unwrap();
    assert!((result - 3.14).abs() < 0.001);
}

#[test]
fn test_float_from_int() {
    let params = make_params(empty_input());
    let result: f64 = starlark_eval("float(42)", &params).unwrap();
    assert_eq!(result, 42.0);
}

#[test]
fn test_round_down() {
    let params = make_params(empty_input());
    let result: i64 = starlark_eval("round(3.4)", &params).unwrap();
    assert_eq!(result, 3);
}

#[test]
fn test_round_up() {
    let params = make_params(empty_input());
    let result: i64 = starlark_eval("round(3.6)", &params).unwrap();
    assert_eq!(result, 4);
}

// ==================== TESTS FOR AVERAGING ====================

#[test]
fn test_average_simple() {
    let params = make_params(empty_input());
    let result: f64 =
        starlark_eval("sum([2, 4, 6]) / len([2, 4, 6])", &params).unwrap();
    assert_eq!(result, 4.0);
}

#[test]
fn test_average_from_input() {
    let input = obj(vec![(
        "scores",
        arr(vec![
            InputValue::Number(0.8),
            InputValue::Number(0.6),
            InputValue::Number(0.9),
            InputValue::Number(0.7),
        ]),
    )]);
    let params = make_params(input);
    let result: f64 =
        starlark_eval("sum(input['scores']) / len(input['scores'])", &params)
            .unwrap();
    assert_eq!(result, 0.75);
}

#[test]
fn test_average_mapped_outputs() {
    let input = empty_input();
    let output = TaskOutputOwned::Vector(vec![dec!(0.2), dec!(0.4), dec!(0.6)]);
    let params = make_params_with_output(input, output);

    let result: f64 =
        starlark_eval("sum(output) / len(output)", &params).unwrap();
    assert!((result - 0.4).abs() < 0.0001);
}

// ==================== TESTS FOR L1 NORMALIZATION ====================

#[test]
fn test_l1_normalize_simple() {
    let params = make_params(empty_input());
    let result: Vec<f64> = starlark_eval(
        "[x / sum([abs(y) for y in [2, 3, 5]]) for x in [2, 3, 5]]",
        &params,
    )
    .unwrap();
    assert_eq!(result.len(), 3);
    assert!((result[0] - 0.2).abs() < 1e-9);
    assert!((result[1] - 0.3).abs() < 1e-9);
    assert!((result[2] - 0.5).abs() < 1e-9);
}

#[test]
fn test_l1_normalize_with_negatives() {
    let params = make_params(empty_input());
    let result: Vec<f64> = starlark_eval(
        "[x / sum([abs(y) for y in [-2, 3, -5]]) for x in [-2, 3, -5]]",
        &params,
    )
    .unwrap();
    assert_eq!(result.len(), 3);
    assert!((result[0] - (-0.2)).abs() < 1e-9);
    assert!((result[1] - 0.3).abs() < 1e-9);
    assert!((result[2] - (-0.5)).abs() < 1e-9);
}

#[test]
fn test_l1_normalize_from_input() {
    let input = obj(vec![(
        "weights",
        arr(vec![
            InputValue::Integer(1),
            InputValue::Integer(2),
            InputValue::Integer(2),
        ]),
    )]);
    let params = make_params(input);
    let result: Vec<f64> = starlark_eval(
        "[w / sum(input['weights']) for w in input['weights']]",
        &params,
    )
    .unwrap();
    assert_eq!(result.len(), 3);
    assert!((result[0] - 0.2).abs() < 1e-9);
    assert!((result[1] - 0.4).abs() < 1e-9);
    assert!((result[2] - 0.4).abs() < 1e-9);
}

#[test]
fn test_l1_normalize_sums_to_one() {
    let input = obj(vec![(
        "values",
        arr(vec![
            InputValue::Number(0.3),
            InputValue::Number(0.5),
            InputValue::Number(0.1),
            InputValue::Number(0.4),
        ]),
    )]);
    let params = make_params(input);
    let result: f64 = starlark_eval(
        "sum([v / sum(input['values']) for v in input['values']])",
        &params,
    )
    .unwrap();
    assert!((result - 1.0).abs() < 0.0001);
}

#[test]
fn test_starlark_primitive_bool() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq("True", &params, &true);
    assert_starlark_deep_eq("False", &params, &false);
}

#[test]
fn test_starlark_primitive_i64() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq("42", &params, &42i64);
}

#[test]
fn test_starlark_primitive_u64() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq("100", &params, &100u64);
}

#[test]
fn test_starlark_primitive_f64() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq("2.5", &params, &2.5f64);
}

#[test]
fn test_starlark_primitive_string() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq("\"world\"", &params, &"world".to_string());
}

#[test]
fn test_starlark_option_none() {
    let params = make_params(empty_input());
    let expected: Option<i64> = None;
    assert_starlark_deep_eq("None", &params, &expected);
}

#[test]
fn test_starlark_option_some() {
    let params = make_params(empty_input());
    let expected: Option<String> = Some("x".to_string());
    assert_starlark_deep_eq("\"x\"", &params, &expected);
}

#[test]
fn test_starlark_vec() {
    let params = make_params(empty_input());
    let expected: Vec<f64> = vec![1.0, 2.0, 3.0];
    assert_starlark_deep_eq("[1.0, 2.0, 3.0]", &params, &expected);
}

#[test]
fn test_starlark_input_boolean() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq("True", &params, &InputValue::Boolean(true));
    assert_starlark_deep_eq("False", &params, &InputValue::Boolean(false));
}

#[test]
fn test_starlark_input_integer() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq("42", &params, &InputValue::Integer(42));
}

#[test]
fn test_starlark_input_number() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq("3.14", &params, &InputValue::Number(3.14));
}

#[test]
fn test_starlark_input_string() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq(
        "\"hello\"",
        &params,
        &InputValue::String("hello".to_string()),
    );
}

#[test]
fn test_starlark_input_array() {
    let params = make_params(empty_input());
    let expected = InputValue::Array(vec![
        InputValue::Integer(1),
        InputValue::Integer(2),
        InputValue::String("x".to_string()),
    ]);
    assert_starlark_deep_eq("[1, 2, \"x\"]", &params, &expected);
}

#[test]
fn test_starlark_input_object() {
    let params = make_params(empty_input());
    let expected = obj(vec![
        ("a", InputValue::Integer(1)),
        ("b", InputValue::String("two".to_string())),
    ]);
    assert_starlark_deep_eq("{\"a\": 1, \"b\": \"two\"}", &params, &expected);
}

#[test]
fn test_starlark_input_expression_boolean() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq(
        "True",
        &params,
        &InputValueExpression::Boolean(true),
    );
}

#[test]
fn test_starlark_input_expression_integer() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq("0", &params, &InputValueExpression::Integer(0));
}

#[test]
fn test_starlark_input_expression_number() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq("1.5", &params, &InputValueExpression::Number(1.5));
}

#[test]
fn test_starlark_input_expression_string() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq(
        "\"s\"",
        &params,
        &InputValueExpression::String("s".to_string()),
    );
}

#[test]
fn test_starlark_input_expression_array() {
    let params = make_params(empty_input());
    let expected = InputValueExpression::Array(vec![
        WithExpression::Value(InputValueExpression::Integer(1)),
        WithExpression::Value(InputValueExpression::String("y".to_string())),
    ]);
    assert_starlark_deep_eq("[1, \"y\"]", &params, &expected);
}

#[test]
fn test_starlark_input_expression_object() {
    let params = make_params(empty_input());
    let mut map = IndexMap::new();
    map.insert(
        "k".to_string(),
        WithExpression::Value(InputValueExpression::Boolean(true)),
    );
    let expected = InputValueExpression::Object(map);
    assert_starlark_deep_eq("{\"k\": True}", &params, &expected);
}

#[test]
fn test_starlark_task_output_scalar() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq(
        "0.5",
        &params,
        &TaskOutputOwned::Scalar(dec!(0.5)),
    );
}

#[test]
fn test_starlark_task_output_scalar_int() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq("1", &params, &TaskOutputOwned::Scalar(dec!(1)));
}

#[test]
fn test_starlark_task_output_vector() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq(
        "[0.25, 0.75]",
        &params,
        &TaskOutputOwned::Vector(vec![dec!(0.25), dec!(0.75)]),
    );
}

#[test]
fn test_starlark_task_output_err() {
    // The Err variant exposes its inner `error` value to Starlark expressions
    // (the `{"error": ...}` wrapping is only on the JSON wire; expression
    // authors see the bare payload).
    let params = make_params(empty_input());
    let err_val = serde_json::Value::String("bad".to_string());
    assert_starlark_deep_eq(
        "\"bad\"",
        &params,
        &TaskOutputOwned::Err { error: err_val },
    );
}

#[test]
fn test_starlark_rich_content_expression_text() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq(
        "\"hi\"",
        &params,
        &RichContentExpression::Text("hi".to_string()),
    );
}

#[test]
fn test_starlark_rich_content_expression_parts() {
    let params = make_params(empty_input());
    let expected = RichContentExpression::Parts(vec![WithExpression::Value(
        RichContentPartExpression::Text {
            text: WithExpression::Value("x".to_string()),
        },
    )]);
    assert_starlark_deep_eq(
        "[{\"type\": \"text\", \"text\": \"x\"}]",
        &params,
        &expected,
    );
}

#[test]
fn test_starlark_image_url_no_detail() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq(
        "{\"url\": \"https://x.com/img.png\"}",
        &params,
        &ImageUrl {
            url: "https://x.com/img.png".to_string(),
            detail: None,
        },
    );
}

#[test]
fn test_starlark_image_url_detail_auto() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq(
        "{\"url\": \"u\", \"detail\": \"auto\"}",
        &params,
        &ImageUrl {
            url: "u".to_string(),
            detail: Some(ImageUrlDetail::Auto),
        },
    );
}

#[test]
fn test_starlark_image_url_detail_low() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq(
        "{\"url\": \"u\", \"detail\": \"low\"}",
        &params,
        &ImageUrl {
            url: "u".to_string(),
            detail: Some(ImageUrlDetail::Low),
        },
    );
}

#[test]
fn test_starlark_image_url_detail_high() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq(
        "{\"url\": \"u\", \"detail\": \"high\"}",
        &params,
        &ImageUrl {
            url: "u".to_string(),
            detail: Some(ImageUrlDetail::High),
        },
    );
}

#[test]
fn test_starlark_input_audio() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq(
        "{\"data\": \"base64data\", \"format\": \"wav\"}",
        &params,
        &InputAudio {
            data: "base64data".to_string(),
            format: "wav".to_string(),
        },
    );
}

#[test]
fn test_starlark_input_audio_defaults() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq(
        "{}",
        &params,
        &InputAudio {
            data: String::new(),
            format: String::new(),
        },
    );
}

#[test]
fn test_starlark_video_url() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq(
        "{\"url\": \"https://v.com/x.mp4\"}",
        &params,
        &VideoUrl {
            url: "https://v.com/x.mp4".to_string(),
        },
    );
}

#[test]
fn test_starlark_file_all_missing() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq(
        "{}",
        &params,
        &File {
            file_data: None,
            file_id: None,
            filename: None,
            file_url: None,
        },
    );
}

#[test]
fn test_starlark_file_with_file_data() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq(
        "{\"file_data\": \"abc\"}",
        &params,
        &File {
            file_data: Some("abc".to_string()),
            file_id: None,
            filename: None,
            file_url: None,
        },
    );
}

#[test]
fn test_starlark_file_with_file_id() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq(
        "{\"file_id\": \"id-1\"}",
        &params,
        &File {
            file_data: None,
            file_id: Some("id-1".to_string()),
            filename: None,
            file_url: None,
        },
    );
}

#[test]
fn test_starlark_file_with_filename() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq(
        "{\"filename\": \"doc.pdf\"}",
        &params,
        &File {
            file_data: None,
            file_id: None,
            filename: Some("doc.pdf".to_string()),
            file_url: None,
        },
    );
}

#[test]
fn test_starlark_file_with_file_url() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq(
        "{\"file_url\": \"https://f.com/f\"}",
        &params,
        &File {
            file_data: None,
            file_id: None,
            filename: None,
            file_url: Some("https://f.com/f".to_string()),
        },
    );
}

#[test]
fn test_starlark_file_all_present() {
    let params = make_params(empty_input());
    assert_starlark_deep_eq(
        "{\"file_data\": \"d\", \"file_id\": \"i\", \"filename\": \"n\", \"file_url\": \"u\"}",
        &params,
        &File {
            file_data: Some("d".to_string()),
            file_id: Some("i".to_string()),
            filename: Some("n".to_string()),
            file_url: Some("u".to_string()),
        },
    );
}

#[test]
fn test_starlark_rich_content_part_text() {
    let params = make_params(empty_input());
    let expected = RichContentPartExpression::Text {
        text: WithExpression::Value("t".to_string()),
    };
    assert_starlark_deep_eq(
        "{\"type\": \"text\", \"text\": \"t\"}",
        &params,
        &expected,
    );
}

#[test]
fn test_starlark_rich_content_part_image_url() {
    let params = make_params(empty_input());
    let expected = RichContentPartExpression::ImageUrl {
        image_url: WithExpression::Value(ImageUrl {
            url: "u".to_string(),
            detail: Some(ImageUrlDetail::High),
        }),
    };
    assert_starlark_deep_eq(
        "{\"type\": \"image_url\", \"image_url\": {\"url\": \"u\", \"detail\": \"high\"}}",
        &params,
        &expected,
    );
}

#[test]
fn test_starlark_rich_content_part_input_audio() {
    let params = make_params(empty_input());
    let expected = RichContentPartExpression::InputAudio {
        input_audio: WithExpression::Value(InputAudio {
            data: "d".to_string(),
            format: "mp3".to_string(),
        }),
    };
    assert_starlark_deep_eq(
        "{\"type\": \"input_audio\", \"input_audio\": {\"data\": \"d\", \"format\": \"mp3\"}}",
        &params,
        &expected,
    );
}

#[test]
fn test_starlark_rich_content_part_input_video() {
    let params = make_params(empty_input());
    let expected = RichContentPartExpression::InputVideo {
        video_url: WithExpression::Value(VideoUrl {
            url: "https://v".to_string(),
        }),
    };
    assert_starlark_deep_eq(
        "{\"type\": \"input_video\", \"video_url\": {\"url\": \"https://v\"}}",
        &params,
        &expected,
    );
}

#[test]
fn test_starlark_rich_content_part_video_url() {
    let params = make_params(empty_input());
    let expected = RichContentPartExpression::VideoUrl {
        video_url: WithExpression::Value(VideoUrl {
            url: "https://v2".to_string(),
        }),
    };
    assert_starlark_deep_eq(
        "{\"type\": \"video_url\", \"video_url\": {\"url\": \"https://v2\"}}",
        &params,
        &expected,
    );
}

#[test]
fn test_starlark_rich_content_part_file() {
    let params = make_params(empty_input());
    let expected = RichContentPartExpression::File {
        file: WithExpression::Value(File {
            file_data: None,
            file_id: Some("fid".to_string()),
            filename: None,
            file_url: None,
        }),
    };
    assert_starlark_deep_eq(
        "{\"type\": \"file\", \"file\": {\"file_id\": \"fid\"}}",
        &params,
        &expected,
    );
}

#[test]
fn test_starlark_assistant_tool_call_function_expression() {
    let params = make_params(empty_input());
    let expected = AssistantToolCallFunctionExpression {
        name: WithExpression::Value("fn".to_string()),
        arguments: WithExpression::Value("{\"x\": 1}".to_string()),
    };
    assert_starlark_deep_eq(
        "{\"name\": \"fn\", \"arguments\": \"{\\\"x\\\": 1}\"}",
        &params,
        &expected,
    );
}

#[test]
fn test_starlark_assistant_tool_call_expression() {
    let params = make_params(empty_input());
    let expected = AssistantToolCallExpression::Function {
        id: WithExpression::Value("call_1".to_string()),
        function: WithExpression::Value(AssistantToolCallFunctionExpression {
            name: WithExpression::Value("f".to_string()),
            arguments: WithExpression::Value("{}".to_string()),
        }),
    };
    assert_starlark_deep_eq(
        "{\"type\": \"function\", \"id\": \"call_1\", \"function\": {\"name\": \"f\", \"arguments\": \"{}\"}}",
        &params,
        &expected,
    );
}

#[test]
fn test_starlark_assistant_tool_call_expression_id_default() {
    let params = make_params(empty_input());
    let expected = AssistantToolCallExpression::Function {
        id: WithExpression::Value(String::new()),
        function: WithExpression::Value(AssistantToolCallFunctionExpression {
            name: WithExpression::Value("g".to_string()),
            arguments: WithExpression::Value("{}".to_string()),
        }),
    };
    assert_starlark_deep_eq(
        "{\"type\": \"function\", \"function\": {\"name\": \"g\", \"arguments\": \"{}\"}}",
        &params,
        &expected,
    );
}

#[test]
fn test_starlark_message_user() {
    let params = make_params(empty_input());
    let expected = MessageExpression::User(UserMessageExpression {
        content: WithExpression::Value(RichContentExpression::Text(
            "hi".to_string(),
        )),
    });
    assert_starlark_deep_eq(
        "{\"role\": \"user\", \"content\": \"hi\"}",
        &params,
        &expected,
    );
}

#[test]
fn test_starlark_message_assistant_content_none() {
    let params = make_params(empty_input());
    let expected = MessageExpression::Assistant(AssistantMessageExpression {
        content: WithExpression::Value(None),
        refusal: WithExpression::Value(None),
        tool_calls: WithExpression::Value(None),
        reasoning: WithExpression::Value(None),
    });
    assert_starlark_deep_eq(
        "{\"role\": \"assistant\", \"content\": None}",
        &params,
        &expected,
    );
}

#[test]
fn test_starlark_message_assistant_with_content() {
    let params = make_params(empty_input());
    let expected = MessageExpression::Assistant(AssistantMessageExpression {
        content: WithExpression::Value(Some(RichContentExpression::Text(
            "ok".to_string(),
        ))),
        refusal: WithExpression::Value(None),
        tool_calls: WithExpression::Value(None),
        reasoning: WithExpression::Value(None),
    });
    assert_starlark_deep_eq(
        "{\"role\": \"assistant\", \"content\": \"ok\"}",
        &params,
        &expected,
    );
}

#[test]
fn test_starlark_message_assistant_with_refusal() {
    let params = make_params(empty_input());
    let expected = MessageExpression::Assistant(AssistantMessageExpression {
        content: WithExpression::Value(None),
        refusal: WithExpression::Value(Some("declined".to_string())),
        tool_calls: WithExpression::Value(None),
        reasoning: WithExpression::Value(None),
    });
    assert_starlark_deep_eq(
        "{\"role\": \"assistant\", \"content\": None, \"refusal\": \"declined\"}",
        &params,
        &expected,
    );
}

#[test]
fn test_starlark_message_tool() {
    let params = make_params(empty_input());
    let expected = MessageExpression::Tool(ToolMessageExpression {
        tool_call_id: WithExpression::Value("tid".to_string()),
        content: WithExpression::Value(RichContentExpression::Text(
            "result".to_string(),
        )),
    });
    assert_starlark_deep_eq(
        "{\"role\": \"tool\", \"tool_call_id\": \"tid\", \"content\": \"result\"}",
        &params,
        &expected,
    );
}
