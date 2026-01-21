//! JMESPath runtime with custom functions for expression evaluation.
//!
//! Extends the standard JMESPath runtime with additional functions:
//! - `add(a, b)` - Addition
//! - `subtract(a, b)` - Subtraction
//! - `multiply(a, b)` - Multiplication
//! - `divide(a, b)` - Division (returns null if dividing by zero)
//! - `mod(a, b)` - Modulo (returns null if dividing by zero)
//! - `json_parse(s)` - Parse a JSON string
//! - `is_null(v)` - Check if a value is null
//! - `if(cond, then, else)` - Conditional expression

use crate::chat;
use jmespath::{
    Context, ErrorReason, JmespathError, Rcvar, RuntimeError, Variable,
    ast::Ast,
};
use std::{collections::BTreeMap, sync::LazyLock};

/// Global JMESPath runtime instance with custom functions.
pub static JMESPATH_RUNTIME: LazyLock<jmespath::Runtime> = LazyLock::new(
    || {
        use jmespath::{
            Context, ErrorReason, JmespathError, Rcvar, Runtime, RuntimeError,
            Variable,
            functions::{ArgumentType, CustomFunction, Signature},
        };
        use serde_json::Number;
        use std::rc::Rc;

        // convert arg
        fn arg_as_number(
            arg: &Rcvar,
            ctx: &Context,
            position: usize,
        ) -> Result<f64, JmespathError> {
            arg.as_number().ok_or_else(|| {
                JmespathError::new(
                    ctx.expression,
                    ctx.offset,
                    ErrorReason::Runtime(RuntimeError::InvalidType {
                        expected: "number".to_string(),
                        actual: arg.get_type().to_string(),
                        position,
                    }),
                )
            })
        }
        fn arg_as_string(
            arg: &Rcvar,
            ctx: &Context,
            position: usize,
        ) -> Result<String, JmespathError> {
            arg.as_string().cloned().ok_or_else(|| {
                JmespathError::new(
                    ctx.expression,
                    ctx.offset,
                    ErrorReason::Runtime(RuntimeError::InvalidType {
                        expected: "string".to_string(),
                        actual: arg.get_type().to_string(),
                        position,
                    }),
                )
            })
        }
        fn arg_as_bool(
            arg: &Rcvar,
            ctx: &Context,
            position: usize,
        ) -> Result<bool, JmespathError> {
            arg.as_boolean().ok_or_else(|| {
                JmespathError::new(
                    ctx.expression,
                    ctx.offset,
                    ErrorReason::Runtime(RuntimeError::InvalidType {
                        expected: "boolean".to_string(),
                        actual: arg.get_type().to_string(),
                        position,
                    }),
                )
            })
        }
        fn arg_as_array<'var>(
            arg: &'var Rcvar,
            ctx: &Context,
            position: usize,
        ) -> Result<&'var Vec<Rcvar>, JmespathError> {
            arg.as_array().ok_or_else(|| {
                JmespathError::new(
                    ctx.expression,
                    ctx.offset,
                    ErrorReason::Runtime(RuntimeError::InvalidType {
                        expected: "array".to_string(),
                        actual: arg.get_type().to_string(),
                        position,
                    }),
                )
            })
        }

        // extract arg
        fn any_arg(
            args: &[Rcvar],
            ctx: &Context,
            position: usize,
            expect_args_len: usize,
        ) -> Result<Rcvar, JmespathError> {
            args.get(position)
                .ok_or_else(|| {
                    JmespathError::new(
                        ctx.expression,
                        ctx.offset,
                        ErrorReason::Runtime(
                            RuntimeError::NotEnoughArguments {
                                expected: expect_args_len,
                                actual: args.len(),
                            },
                        ),
                    )
                })
                .cloned()
        }
        fn number_arg(
            args: &[Rcvar],
            ctx: &Context,
            position: usize,
            expect_args_len: usize,
        ) -> Result<f64, JmespathError> {
            let arg = any_arg(args, ctx, position, expect_args_len)?;
            arg_as_number(&arg, ctx, position)
        }
        fn nullable_number_arg(
            args: &[Rcvar],
            ctx: &Context,
            position: usize,
            expect_args_len: usize,
        ) -> Result<Option<f64>, JmespathError> {
            let arg = any_arg(args, ctx, position, expect_args_len)?;
            if arg.is_null() {
                Ok(None)
            } else {
                Ok(Some(arg_as_number(&arg, ctx, position)?))
            }
        }
        fn string_arg(
            args: &[Rcvar],
            ctx: &Context,
            position: usize,
            expect_args_len: usize,
        ) -> Result<String, JmespathError> {
            let arg = any_arg(args, ctx, position, expect_args_len)?;
            arg_as_string(&arg, ctx, position)
        }
        fn nullable_string_arg(
            args: &[Rcvar],
            ctx: &Context,
            position: usize,
            expect_args_len: usize,
        ) -> Result<Option<String>, JmespathError> {
            let arg = any_arg(args, ctx, position, expect_args_len)?;
            if arg.is_null() {
                Ok(None)
            } else {
                Ok(Some(arg_as_string(&arg, ctx, position)?))
            }
        }
        fn bool_arg(
            args: &[Rcvar],
            ctx: &Context,
            position: usize,
            expect_args_len: usize,
        ) -> Result<bool, JmespathError> {
            let arg = any_arg(args, ctx, position, expect_args_len)?;
            arg_as_bool(&arg, ctx, position)
        }
        fn nullable_bool_arg(
            args: &[Rcvar],
            ctx: &Context,
            position: usize,
            expect_args_len: usize,
        ) -> Result<Option<bool>, JmespathError> {
            let arg = any_arg(args, ctx, position, expect_args_len)?;
            if arg.is_null() {
                Ok(None)
            } else {
                Ok(Some(arg_as_bool(&arg, ctx, position)?))
            }
        }
        fn array_arg(
            args: &[Rcvar],
            ctx: &Context,
            position: usize,
            expect_args_len: usize,
        ) -> Result<Vec<Rcvar>, JmespathError> {
            let arg = any_arg(args, ctx, position, expect_args_len)?;
            let array = arg_as_array(&arg, ctx, position)?;
            Ok(array.clone())
        }
        fn number_array_arg(
            args: &[Rcvar],
            ctx: &Context,
            position: usize,
            expect_args_len: usize,
        ) -> Result<Vec<f64>, JmespathError> {
            let arg = any_arg(args, ctx, position, expect_args_len)?;
            let array = arg_as_array(&arg, ctx, position)?;
            let mut numbers = Vec::with_capacity(array.len());
            for item in array.iter() {
                let number = arg_as_number(item, ctx, position)?;
                numbers.push(number);
            }
            Ok(numbers)
        }

        // return value
        fn rcvar_f64(n: f64) -> Rcvar {
            Rc::new(Variable::Number(
                Number::from_f64(n).unwrap_or(Number::from_f64(0.0).unwrap()),
            ))
        }
        fn rcvar_f64_u64(n: f64) -> Rcvar {
            Rc::new(Variable::Number(Number::from(n.round() as u64)))
        }

        let mut runtime = Runtime::new();

        // https://jmespath.org/specification.html
        runtime.register_builtin_functions();

        // basic math
        runtime.register_function(
            "add",
            Box::new(CustomFunction::new(
                Signature::new(
                    vec![ArgumentType::Number, ArgumentType::Number],
                    None,
                ),
                Box::new(|args: &[Rcvar], ctx: &mut Context| {
                    let a = number_arg(args, ctx, 0, 2)?;
                    let b = number_arg(args, ctx, 1, 2)?;
                    Ok(rcvar_f64(a + b))
                }),
            )),
        );
        runtime.register_function(
            "subtract",
            Box::new(CustomFunction::new(
                Signature::new(
                    vec![ArgumentType::Number, ArgumentType::Number],
                    None,
                ),
                Box::new(|args: &[Rcvar], ctx: &mut Context| {
                    let a = number_arg(args, ctx, 0, 2)?;
                    let b = number_arg(args, ctx, 1, 2)?;
                    Ok(rcvar_f64(a - b))
                }),
            )),
        );
        runtime.register_function(
            "multiply",
            Box::new(CustomFunction::new(
                Signature::new(
                    vec![ArgumentType::Number, ArgumentType::Number],
                    None,
                ),
                Box::new(|args: &[Rcvar], ctx: &mut Context| {
                    let a = number_arg(args, ctx, 0, 2)?;
                    let b = number_arg(args, ctx, 1, 2)?;
                    Ok(rcvar_f64(a * b))
                }),
            )),
        );
        runtime.register_function(
            "divide",
            Box::new(CustomFunction::new(
                Signature::new(
                    vec![ArgumentType::Number, ArgumentType::Number],
                    None,
                ),
                Box::new(|args: &[Rcvar], ctx: &mut Context| {
                    let a = number_arg(args, ctx, 0, 2)?;
                    let b = number_arg(args, ctx, 1, 2)?;
                    if b == 0.0 {
                        Ok(Rc::new(Variable::Null))
                    } else {
                        Ok(rcvar_f64(a / b))
                    }
                }),
            )),
        );
        runtime.register_function(
            "mod",
            Box::new(CustomFunction::new(
                Signature::new(
                    vec![ArgumentType::Number, ArgumentType::Number],
                    None,
                ),
                Box::new(|args: &[Rcvar], ctx: &mut Context| {
                    let a = number_arg(args, ctx, 0, 2)?;
                    let b = number_arg(args, ctx, 1, 2)?;
                    if b == 0.0 {
                        Ok(Rc::new(Variable::Null))
                    } else {
                        Ok(rcvar_f64(a % b))
                    }
                }),
            )),
        );

        // parse JSON into native JMESPath variable
        runtime.register_function(
            "json_parse",
            Box::new(CustomFunction::new(
                Signature::new(vec![ArgumentType::String], None),
                Box::new(|args: &[Rcvar], ctx: &mut Context| {
                    let string = string_arg(args, ctx, 0, 1)?;
                    let variable: jmespath::Variable =
                        serde_json::from_str(&string)
                            .unwrap_or(jmespath::Variable::Null);
                    Ok(Rc::new(variable))
                }),
            )),
        );

        // check if value is null
        runtime.register_function(
            "is_null",
            Box::new(CustomFunction::new(
                Signature::new(vec![ArgumentType::Any], None),
                Box::new(|args: &[Rcvar], ctx: &mut Context| {
                    let value = any_arg(args, ctx, 0, 1)?;
                    Ok(Rc::new(Variable::Bool(value.is_null())))
                }),
            )),
        );

        // conditional expression
        // if(cond, then, else)
        runtime.register_function(
            "if",
            Box::new(CustomFunction::new(
                Signature::new(
                    vec![
                        ArgumentType::Any,
                        ArgumentType::Any,
                        ArgumentType::Any,
                    ],
                    None,
                ),
                Box::new(|args: &[Rcvar], ctx: &mut Context| {
                    let condition = any_arg(args, ctx, 0, 3)?;
                    let then_branch = any_arg(args, ctx, 1, 3)?;
                    let else_branch = any_arg(args, ctx, 2, 3)?;
                    if condition.is_truthy() {
                        Ok(then_branch)
                    } else {
                        Ok(else_branch)
                    }
                }),
            )),
        );

        // input_value_switch function
        // selects output based on the type of the input value
        // evaluates exprefs if necessary
        runtime.register_function(
            "input_value_switch",
            Box::new(CustomFunction::new(
                Signature::new(
                    vec![
                        ArgumentType::Any, // Input Value
                        ArgumentType::Any, // Object
                        ArgumentType::Any, // Array
                        ArgumentType::Any, // String
                        ArgumentType::Any, // Integer
                        ArgumentType::Any, // Number
                        ArgumentType::Any, // Boolean
                        ArgumentType::Any, // Image
                        ArgumentType::Any, // Audio
                        ArgumentType::Any, // Video
                        ArgumentType::Any, // File
                    ],
                    None,
                ),
                Box::new(|args: &[Rcvar], ctx: &mut Context| {
                    // first arg is input value
                    let input_value = any_arg(args, ctx, 0, 11)?;
                    // output_value depends on the type of input_value
                    let output_value = match serde_json::from_value::<
                        super::Input,
                    >(
                        serde_json::to_value(&input_value)
                            .unwrap_or(serde_json::Value::Null),
                    ) {
                        Ok(super::Input::Object(_)) => Ok(
                            any_arg(args, ctx, 1, 11)?,
                        ),
                        Ok(super::Input::Array(_)) => Ok(
                            any_arg(args, ctx, 2, 11)?,
                        ),
                        Ok(super::Input::String(_)) => Ok(
                            any_arg(args, ctx, 3, 11)?,
                        ),
                        Ok(super::Input::Integer(_)) => Ok(
                            any_arg(args, ctx, 4, 11)?,
                        ),
                        Ok(super::Input::Number(_)) => Ok(
                            any_arg(args, ctx, 5, 11)?,
                        ),
                        Ok(super::Input::Boolean(_)) => Ok(
                            any_arg(args, ctx, 6, 11)?,
                        ),
                        Ok(super::Input::RichContentPart(
                            chat::completions::request::RichContentPart::ImageUrl {
                                ..
                            }
                        )) => Ok(any_arg(args, ctx, 7, 11)?),
                        Ok(super::Input::RichContentPart(
                            chat::completions::request::RichContentPart::InputAudio {
                                ..
                            }
                        )) => Ok(any_arg(args, ctx, 8, 11)?),
                        Ok(super::Input::RichContentPart(
                            chat::completions::request::RichContentPart::InputVideo {
                                ..
                            }
                        )) => Ok(any_arg(args, ctx, 9, 11)?),
                        Ok(super::Input::RichContentPart(
                            chat::completions::request::RichContentPart::VideoUrl {
                                ..
                            }
                        )) => Ok(any_arg(args, ctx, 9, 11)?),
                        Ok(super::Input::RichContentPart(
                            chat::completions::request::RichContentPart::File {
                                ..
                            }
                        )) => Ok(any_arg(args, ctx, 10, 11)?),
                        Ok(super::Input::RichContentPart(
                            chat::completions::request::RichContentPart::Text {
                                ..
                            }
                        )) => Ok(any_arg(args, ctx, 1, 11)?),
                        Err(_) => Err(JmespathError::new(
                            ctx.expression,
                            ctx.offset,
                            ErrorReason::Runtime(RuntimeError::InvalidType {
                                expected: "valid Input type".to_string(),
                                actual: input_value.get_type().to_string(),
                                position: 0,
                            }),
                        )),
                    }?;
                    if let Some(ast) = output_value.as_expref() {
                        interpret(&input_value, &ast, ctx)
                    } else {
                        Ok(output_value)
                    }
                }),
            )),
        );

        // zips a 2D array and maps each column with an expref
        // if sub-arrays are of different lengths, fills missing values with null
        runtime.register_function(
            "zip_map",
            Box::new(CustomFunction::new(
                Signature::new(
                    vec![
                        ArgumentType::Expref,
                        ArgumentType::TypedArray(Box::new(ArgumentType::Array)),
                    ],
                    None,
                ),
                Box::new(|args: &[Rcvar], ctx: &mut Context| {
                    let expref = args[0].as_expref().unwrap();
                    let input_array = args[1].as_array().unwrap();
                    let mut output_array = Vec::with_capacity(
                        input_array
                            .iter()
                            .map(|v| v.as_array().unwrap().len())
                            .max()
                            .unwrap_or_default(),
                    );
                    for i in 0..output_array.capacity() {
                        let mut column = Vec::with_capacity(input_array.len());
                        for row in input_array.iter() {
                            let row_array = row.as_array().unwrap();
                            if let Some(value) = row_array.get(i) {
                                column.push(value.clone());
                            } else {
                                column.push(Rc::new(Variable::Null));
                            }
                        }
                        output_array.push(interpret(
                            &Rc::new(Variable::Array(column)),
                            &expref,
                            ctx,
                        )?);
                    }
                    Ok(Rc::new(Variable::Array(output_array)))
                }),
            )),
        );

        // L1 normalization of a number array
        runtime.register_function(
            "l1_normalize",
            Box::new(CustomFunction::new(
                Signature::new(
                    vec![ArgumentType::TypedArray(Box::new(
                        ArgumentType::Number,
                    ))],
                    None,
                ),
                Box::new(|args: &[Rcvar], ctx: &mut Context| {
                    let numbers = number_array_arg(args, ctx, 0, 1)?;
                    let sum: f64 = numbers.iter().map(|n| n.abs()).sum();
                    if numbers.len() == 0 {
                        Ok(Rc::new(Variable::Array(Vec::new())))
                    } else if sum == 0.0 {
                        Ok(Rc::new(Variable::Array(
                            numbers
                                .iter()
                                .map(|_| {
                                    Rc::new(Variable::Number(
                                        Number::from_f64(
                                            1.0 / numbers.len() as f64,
                                        )
                                        .unwrap(),
                                    ))
                                })
                                .collect(),
                        )))
                    } else {
                        Ok(Rc::new(Variable::Array(
                            numbers
                                .iter()
                                .map(|n| rcvar_f64(n / sum))
                                .collect(),
                        )))
                    }
                }),
            )),
        );

        runtime
    },
);

/// Copied from jmespath::interpreter
/// https://github.com/jmespath/jmespath.rs/pull/60
/// Interprets the given data using an AST node.
fn interpret(
    data: &Rcvar,
    node: &Ast,
    ctx: &mut Context<'_>,
) -> Result<Rcvar, JmespathError> {
    match *node {
        Ast::Field { ref name, .. } => Ok(data.get_field(name)),
        Ast::Subexpr {
            ref lhs, ref rhs, ..
        } => {
            let left_result = interpret(data, lhs, ctx)?;
            interpret(&left_result, rhs, ctx)
        }
        Ast::Identity { .. } => Ok(data.clone()),
        Ast::Literal { ref value, .. } => Ok(value.clone()),
        Ast::Index { idx, .. } => {
            if idx >= 0 {
                Ok(data.get_index(idx as usize))
            } else {
                Ok(data.get_negative_index((-idx) as usize))
            }
        }
        Ast::Or {
            ref lhs, ref rhs, ..
        } => {
            let left = interpret(data, lhs, ctx)?;
            if left.is_truthy() {
                Ok(left)
            } else {
                interpret(data, rhs, ctx)
            }
        }
        Ast::And {
            ref lhs, ref rhs, ..
        } => {
            let left = interpret(data, lhs, ctx)?;
            if !left.is_truthy() {
                Ok(left)
            } else {
                interpret(data, rhs, ctx)
            }
        }
        Ast::Not { ref node, .. } => {
            let result = interpret(data, node, ctx)?;
            Ok(Rcvar::new(Variable::Bool(!result.is_truthy())))
        }
        // Returns the resut of RHS if cond yields truthy value.
        Ast::Condition {
            ref predicate,
            ref then,
            ..
        } => {
            let cond_result = interpret(data, predicate, ctx)?;
            if cond_result.is_truthy() {
                interpret(data, then, ctx)
            } else {
                Ok(Rcvar::new(Variable::Null))
            }
        }
        Ast::Comparison {
            ref comparator,
            ref lhs,
            ref rhs,
            ..
        } => {
            let left = interpret(data, lhs, ctx)?;
            let right = interpret(data, rhs, ctx)?;
            Ok(left
                .compare(comparator, &right)
                .map_or(Rcvar::new(Variable::Null), |result| {
                    Rcvar::new(Variable::Bool(result))
                }))
        }
        // Converts an object into a JSON array of its values.
        Ast::ObjectValues { ref node, .. } => {
            let subject = interpret(data, node, ctx)?;
            match *subject {
                Variable::Object(ref v) => Ok(Rcvar::new(Variable::Array(
                    v.values().cloned().collect::<Vec<Rcvar>>(),
                ))),
                _ => Ok(Rcvar::new(Variable::Null)),
            }
        }
        // Passes the results of lhs into rhs if lhs yields an array and
        // each node of lhs that passes through rhs yields a non-null value.
        Ast::Projection {
            ref lhs, ref rhs, ..
        } => match interpret(data, lhs, ctx)?.as_array() {
            None => Ok(Rcvar::new(Variable::Null)),
            Some(left) => {
                let mut collected = vec![];
                for element in left {
                    let current = interpret(element, rhs, ctx)?;
                    if !current.is_null() {
                        collected.push(current);
                    }
                }
                Ok(Rcvar::new(Variable::Array(collected)))
            }
        },
        Ast::Flatten { ref node, .. } => match interpret(data, node, ctx)?
            .as_array()
        {
            None => Ok(Rcvar::new(Variable::Null)),
            Some(a) => {
                let mut collected: Vec<Rcvar> = vec![];
                for element in a {
                    match element.as_array() {
                        Some(array) => collected.extend(array.iter().cloned()),
                        _ => collected.push(element.clone()),
                    }
                }
                Ok(Rcvar::new(Variable::Array(collected)))
            }
        },
        Ast::MultiList { ref elements, .. } => {
            if data.is_null() {
                Ok(Rcvar::new(Variable::Null))
            } else {
                let mut collected = vec![];
                for node in elements {
                    collected.push(interpret(data, node, ctx)?);
                }
                Ok(Rcvar::new(Variable::Array(collected)))
            }
        }
        Ast::MultiHash { ref elements, .. } => {
            if data.is_null() {
                Ok(Rcvar::new(Variable::Null))
            } else {
                let mut collected = BTreeMap::new();
                for kvp in elements {
                    let value = interpret(data, &kvp.value, ctx)?;
                    collected.insert(kvp.key.clone(), value);
                }
                Ok(Rcvar::new(Variable::Object(collected)))
            }
        }
        Ast::Function {
            ref name,
            ref args,
            offset,
        } => {
            let mut fn_args: Vec<Rcvar> = vec![];
            for arg in args {
                fn_args.push(interpret(data, arg, ctx)?);
            }
            // Reset the offset so that it points to the function being evaluated.
            ctx.offset = offset;
            match ctx.runtime.get_function(name) {
                Some(f) => f.evaluate(&fn_args, ctx),
                None => {
                    let reason = ErrorReason::Runtime(
                        RuntimeError::UnknownFunction(name.to_owned()),
                    );
                    Err(JmespathError::from_ctx(ctx, reason))
                }
            }
        }
        Ast::Expref { ref ast, .. } => {
            Ok(Rcvar::new(Variable::Expref(*ast.clone())))
        }
        Ast::Slice {
            start,
            stop,
            step,
            offset,
        } => {
            if step == 0 {
                ctx.offset = offset;
                let reason = ErrorReason::Runtime(RuntimeError::InvalidSlice);
                Err(JmespathError::from_ctx(ctx, reason))
            } else {
                match data.slice(start, stop, step) {
                    Some(array) => Ok(Rcvar::new(Variable::Array(array))),
                    None => Ok(Rcvar::new(Variable::Null)),
                }
            }
        }
    }
}
