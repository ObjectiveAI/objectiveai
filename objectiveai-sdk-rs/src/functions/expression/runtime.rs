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
        #[allow(dead_code)]
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
        #[allow(dead_code)]
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
        #[allow(dead_code)]
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
        #[allow(dead_code)]
        fn string_arg(
            args: &[Rcvar],
            ctx: &Context,
            position: usize,
            expect_args_len: usize,
        ) -> Result<String, JmespathError> {
            let arg = any_arg(args, ctx, position, expect_args_len)?;
            arg_as_string(&arg, ctx, position)
        }
        #[allow(dead_code)]
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
        #[allow(dead_code)]
        fn bool_arg(
            args: &[Rcvar],
            ctx: &Context,
            position: usize,
            expect_args_len: usize,
        ) -> Result<bool, JmespathError> {
            let arg = any_arg(args, ctx, position, expect_args_len)?;
            arg_as_bool(&arg, ctx, position)
        }
        #[allow(dead_code)]
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
        #[allow(dead_code)]
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
        #[allow(dead_code)]
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
                        output_array.push(jmespath::interpret(
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
    });
