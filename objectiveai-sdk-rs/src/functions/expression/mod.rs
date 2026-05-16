//! Expression evaluation engine for Function compilation.
//!
//! This module provides the expression system used by Functions to define
//! dynamic behavior. Expressions are evaluated against input data and task
//! results to produce concrete values.
//!
//! # Supported Languages
//!
//! - **JMESPath** (`{"$jmespath": "..."}`) - JSON query language
//! - **Starlark** (`{"$starlark": "..."}`) - Python-like configuration language
//!
//! # Key Types
//!
//! - [`Expression`] - Either a JMESPath or Starlark expression
//! - [`WithExpression<T>`] - Either a literal value or an expression
//! - [`InputValue`] - The input data structure passed to expressions
//! - [`Params`] - Context available during expression evaluation
//!
//! # Expression Context
//!
//! Expressions can access:
//! - `input` - The function's input data
//! - `tasks` - Results from previously executed tasks
//! - `map` - Current map element (when in mapped task context)

mod error;
mod expression;
mod input_schema;
mod input_value;
mod params;
mod runtime;
mod special;
mod starlark;

pub use error::*;
pub use expression::*;
pub use input_schema::*;
pub use input_value::*;
pub use params::*;
pub use runtime::*;
pub use special::{FromSpecial, Special};
pub(crate) use special::impl_from_special_unsupported;
pub use starlark::{FromStarlarkValue, ToStarlarkValue};

#[cfg(test)]
mod expression_tests;
#[cfg(test)]
mod special_tests;
#[cfg(test)]
mod starlark_tests;
#[cfg(test)]
mod input_schema_tests;
