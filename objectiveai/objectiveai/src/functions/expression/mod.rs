//! JMESPath expression evaluation engine for Function compilation.
//!
//! This module provides the expression system used by Functions to define
//! dynamic behavior. Expressions are evaluated against input data and task
//! results to produce concrete values.
//!
//! # Key Types
//!
//! - [`Expression`] - A JMESPath expression (`{"$jmespath": "..."}`)
//! - [`WithExpression<T>`] - Either a literal value or an expression
//! - [`Input`] - The input data structure passed to expressions
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
mod input;
mod params;
mod runtime;

pub use error::*;
pub use expression::*;
pub use input::*;
pub use params::*;
pub use runtime::*;
