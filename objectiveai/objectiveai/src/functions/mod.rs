//! Function definitions, profiles, and execution types.
//!
//! Functions are composable scoring pipelines that transform structured input
//! into scores. They are the primary interface for most ObjectiveAI users.
//!
//! # Overview
//!
//! A Function consists of:
//! - **Input schema** - Defines expected input structure
//! - **Input maps** - Optional expressions to transform input into arrays for mapped tasks
//! - **Tasks** - A list of operations (Vector Completions or nested Functions)
//! - **Output** - Expression that combines task results into final score(s)
//!
//! # Function Types
//!
//! - [`RemoteFunction`] - GitHub-hosted function with description and schema
//! - [`InlineFunction`] - Inline function definition without metadata
//! - **Scalar** - Produces a single score in [0, 1]
//! - **Vector** - Produces a vector of scores that sums to 1
//!
//! # Task Types
//!
//! - [`ScalarFunctionTask`] - Calls a scalar function
//! - [`VectorFunctionTask`] - Calls a vector function
//! - [`VectorCompletionTask`] - Runs a vector completion
//!
//! # Client-Side Compilation
//!
//! Functions use JMESPath expressions for dynamic behavior. The SDK can compile
//! these expressions client-side to preview results during Function authoring:
//!
//! - [`Function::compile_tasks`] - Resolves task expressions to show final tasks for a given input
//! - [`Function::compile_output`] - Computes the final output given input and task outputs
//!
//! # Submodules
//!
//! - [`executions`] - Function execution request/response types
//! - [`expression`] - JMESPath expression evaluation engine
//! - [`profiles`] - Profile management and computation

pub mod executions;
pub mod expression;
mod function;
mod profile;
pub mod profiles;
pub mod response;
mod task;

pub use function::*;
pub use profile::*;
pub use task::*;

#[cfg(feature = "http")]
mod http;

#[cfg(feature = "http")]
pub use http::*;
