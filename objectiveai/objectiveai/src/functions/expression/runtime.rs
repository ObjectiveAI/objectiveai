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

use std::sync::LazyLock;

/// Global JMESPath runtime instance with custom functions.
pub static JMESPATH_RUNTIME: LazyLock<jmespath::Runtime> =
    LazyLock::new(|| {
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

        // math
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

        // custom stuff
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

        runtime
    });
