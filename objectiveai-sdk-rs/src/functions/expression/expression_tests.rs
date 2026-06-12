use super::*;
use crate::agent::completions::message::{
    AssistantToolCallExpression, AssistantToolCallFunctionExpression, File,
    ImageUrl, InputAudio, MessageExpression, RichContentExpression,
    RichContentPartExpression, SimpleContentExpression,
    SimpleContentPartExpression, VideoUrl,
};
use crate::functions::expression::{
    ExpressionError, InputValue, InputValueExpression, Params, ParamsOwned,
    TaskOutputOwned,
};
use indexmap::IndexMap;

fn empty_params() -> Params<'static, 'static> {
    Params::Owned(ParamsOwned {
        input: InputValue::Object(IndexMap::new()),
        output: None,
        map: None,
    })
}

fn starlark_one<
    T: serde::de::DeserializeOwned
        + crate::functions::expression::starlark::FromStarlarkValue
        + crate::functions::expression::FromSpecial,
>(
    code: &str,
    params: &Params,
) -> T {
    Expression::Starlark(code.to_string())
        .compile_one(params)
        .unwrap()
}

fn params_with_object(
    pairs: Vec<(&str, InputValue)>,
) -> Params<'static, 'static> {
    let mut map = IndexMap::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), v);
    }
    Params::Owned(ParamsOwned {
        input: InputValue::Object(map),
        output: None,
        map: None,
    })
}

#[test]
fn expression_compile_one_string_starlark() {
    let params = params_with_object(vec![(
        "name",
        InputValue::String("alice".to_string()),
    )]);
    let expr = Expression::Starlark("input['name']".to_string());

    let value: String = expr.compile_one(&params).unwrap();
    assert_eq!(value, "alice");
}

#[test]
fn expression_compile_one_or_many_integers_from_array() {
    let params = params_with_object(vec![(
        "numbers",
        InputValue::Array(vec![
            InputValue::Integer(1),
            InputValue::Integer(2),
            InputValue::Integer(3),
        ]),
    )]);
    let expr = Expression::Starlark("input['numbers']".to_string());

    let result: OneOrMany<i64> = expr.compile_one_or_many(&params).unwrap();
    match result {
        OneOrMany::Many(values) => assert_eq!(values, vec![1, 2, 3]),
        other => panic!("expected Many variant, got {:?}", other),
    }
}

#[test]
fn expression_compile_one_or_many_filters_nulls_and_empty_is_many() {
    let params = empty_params();

    // Single null becomes an empty Many
    let expr_null = Expression::Starlark("None".to_string());
    let result: OneOrMany<i64> =
        expr_null.compile_one_or_many(&params).unwrap();
    match result {
        OneOrMany::Many(values) => assert!(values.is_empty()),
        other => panic!("expected Many([]), got {:?}", other),
    }

    // Array with nulls filters them out
    let expr_array = Expression::Starlark("[1, None, 3]".to_string());
    let result: OneOrMany<i64> =
        expr_array.compile_one_or_many(&params).unwrap();
    match result {
        OneOrMany::Many(values) => assert_eq!(values, vec![1, 3]),
        other => panic!("expected Many([1, 3]), got {:?}", other),
    }
}

#[test]
fn expression_compile_one_errors_when_many() {
    let params = empty_params();
    let expr = Expression::Starlark("[1, 2]".to_string());

    let err = expr.compile_one::<i64>(&params).unwrap_err();
    match err {
        ExpressionError::ExpectedOneValueFoundMany => {}
        other => {
            panic!("expected ExpectedOneValueFoundMany, got {:?}", other)
        }
    }
}

#[test]
fn withexpression_literal_passthrough_one_and_one_or_many() {
    let params = empty_params();
    let with_expr = WithExpression::Value::<String>("hello".to_string());

    let one = with_expr.clone().compile_one(&params).unwrap();
    assert_eq!(one, "hello");

    let one_or_many = with_expr.compile_one_or_many(&params).unwrap();
    match one_or_many {
        OneOrMany::One(value) => assert_eq!(value, "hello"),
        other => panic!("expected One(\"hello\"), got {:?}", other),
    }
}

#[test]
fn withexpression_expression_uses_underlying_expression_for_one_and_many() {
    // Use an input with a scalar and an array to exercise both paths.
    let params = params_with_object(vec![
        ("value", InputValue::Integer(42)),
        (
            "values",
            InputValue::Array(vec![
                InputValue::Integer(1),
                InputValue::Integer(2),
                InputValue::Integer(3),
            ]),
        ),
    ]);

    // compile_one with scalar output
    let with_scalar = WithExpression::Expression::<i64>(Expression::Starlark(
        "input['value']".to_string(),
    ));
    let scalar = with_scalar.compile_one(&params).unwrap();
    assert_eq!(scalar, 42);

    // compile_one_or_many with array output
    let with_many = WithExpression::Expression::<i64>(Expression::Starlark(
        "input['values']".to_string(),
    ));
    let result = with_many.compile_one_or_many(&params).unwrap();
    match result {
        OneOrMany::Many(values) => assert_eq!(values, vec![1, 2, 3]),
        other => panic!("expected Many([1, 2, 3]), got {:?}", other),
    }
}

#[test]
fn expression_outputs_primitives_and_options() {
    let params = empty_params();

    let b: bool = starlark_one("True", &params);
    assert!(b);

    let n: u64 = starlark_one("123", &params);
    assert_eq!(n, 123);

    let some_s: Option<String> = starlark_one("\"hi\"", &params);
    assert_eq!(some_s.as_deref(), Some("hi"));
    // Optional values: use a literal `WithExpression::Value(None)` to
    // represent null. A *starlark expression* that evaluates to None is
    // treated as an empty array by `compile_one_or_many`, which makes
    // `compile_one` return ExpectedOneValueFoundMany.
    let none_s: Option<String> = WithExpression::Value::<Option<String>>(None)
        .compile_one(&params)
        .unwrap();
    assert_eq!(none_s, None);
    let err = Expression::Starlark("None".to_string())
        .compile_one::<Option<String>>(&params)
        .unwrap_err();
    assert!(matches!(err, ExpressionError::ExpectedOneValueFoundMany));

    let some_b: Option<bool> = starlark_one("False", &params);
    assert_eq!(some_b, Some(false));
    let none_b: Option<bool> = WithExpression::Value::<Option<bool>>(None)
        .compile_one(&params)
        .unwrap();
    assert_eq!(none_b, None);
    let err = Expression::Starlark("None".to_string())
        .compile_one::<Option<bool>>(&params)
        .unwrap_err();
    assert!(matches!(err, ExpressionError::ExpectedOneValueFoundMany));
}

#[test]
fn expression_outputs_input_and_vec_input() {
    let params = empty_params();

    let input: InputValue = starlark_one("\"hello\"", &params);
    assert!(matches!(input, InputValue::String(s) if s == "hello"));

    let inputs: Vec<InputValue> = starlark_one("[1, \"a\", True]", &params);
    assert_eq!(inputs.len(), 3);
    assert!(matches!(&inputs[0], InputValue::Integer(1)));
    assert!(matches!(&inputs[1], InputValue::String(s) if s == "a"));
    assert!(matches!(&inputs[2], InputValue::Boolean(true)));
}

#[test]
fn expression_outputs_input_expression() {
    let params = empty_params();

    let ie: InputValueExpression = starlark_one("{\"k\": \"v\"}", &params);
    match ie {
        InputValueExpression::Object(obj) => {
            assert!(obj.contains_key("k"));
        }
        other => {
            panic!("expected InputValueExpression::Object, got {:?}", other)
        }
    }
}

#[test]
fn expression_outputs_content_expression_types() {
    let params = empty_params();

    let sc: SimpleContentExpression = starlark_one("\"hello\"", &params);
    assert!(matches!(sc, SimpleContentExpression::Text(t) if t == "hello"));

    let rc: RichContentExpression = starlark_one("\"hello\"", &params);
    assert!(matches!(rc, RichContentExpression::Text(t) if t == "hello"));

    let scp: SimpleContentPartExpression =
        starlark_one("{\"type\": \"text\", \"text\": \"hi\"}", &params);
    match scp {
        SimpleContentPartExpression::Text { .. } => {}
    }

    let rcp: RichContentPartExpression =
        starlark_one("{\"type\": \"text\", \"text\": \"hi\"}", &params);
    match rcp {
        RichContentPartExpression::Text { .. } => {}
        other => {
            panic!("expected RichContentPartExpression::Text, got {:?}", other)
        }
    }
}

#[test]
fn expression_outputs_rich_media_structs() {
    let params = empty_params();

    let image: ImageUrl =
        starlark_one("{\"url\": \"https://example.com/img.png\"}", &params);
    assert_eq!(image.url, "https://example.com/img.png");

    let audio: InputAudio =
        starlark_one("{\"data\": \"BASE64\", \"format\": \"wav\"}", &params);
    assert_eq!(audio.format, "wav");

    let video: VideoUrl =
        starlark_one("{\"url\": \"https://example.com/v.mp4\"}", &params);
    assert_eq!(video.url, "https://example.com/v.mp4");

    let file: File = starlark_one("{\"file_id\": \"file_123\"}", &params);
    assert_eq!(file.file_id.as_deref(), Some("file_123"));
}

#[test]
fn expression_outputs_message_types() {
    let params = empty_params();

    let msg: MessageExpression =
        starlark_one("{\"role\": \"user\", \"content\": \"hi\"}", &params);
    match msg {
        MessageExpression::User(_) => {}
        other => {
            panic!("expected MessageExpression::User, got {:?}", other)
        }
    }

    let messages: Vec<WithExpression<MessageExpression>> = starlark_one(
        "[{\"role\": \"user\", \"content\": \"hi\"}, {\"role\": \"system\", \"content\": \"s\"}]",
        &params,
    );
    assert_eq!(messages.len(), 2);
    assert!(matches!(messages[0], WithExpression::Value(_)));
}

#[test]
fn expression_outputs_assistant_tool_call_types() {
    let params = empty_params();

    let atcf: AssistantToolCallFunctionExpression = starlark_one(
        "{\"name\": \"do_thing\", \"arguments\": \"{}\"}",
        &params,
    );
    assert!(
        matches!(atcf.name, WithExpression::Value(ref s) if s == "do_thing")
    );

    let atc: AssistantToolCallExpression = starlark_one(
        "{\"type\": \"function\", \"id\": \"call_1\", \"function\": {\"name\": \"do_thing\", \"arguments\": \"{}\"}}",
        &params,
    );
    match atc {
        AssistantToolCallExpression::Function { .. } => {}
    }

    let atcs: Vec<WithExpression<AssistantToolCallExpression>> = starlark_one(
        "[{\"type\": \"function\", \"id\": \"call_1\", \"function\": {\"name\": \"do_thing\", \"arguments\": \"{}\"}}]",
        &params,
    );
    assert_eq!(atcs.len(), 1);
}

#[test]
fn expression_outputs_task_output_scalar_and_vector() {
    let params = empty_params();

    let scalar: TaskOutputOwned = starlark_one("0.75", &params);
    match scalar {
        TaskOutputOwned::Scalar(_) => {}
        other => panic!("expected TaskOutputOwned::Scalar, got {:?}", other),
    }

    let vector: TaskOutputOwned = starlark_one("[0.1, 0.2, 0.7]", &params);
    match vector {
        TaskOutputOwned::Vector(v) => assert_eq!(v.len(), 3),
        other => panic!("expected TaskOutputOwned::Vector, got {:?}", other),
    }
}
