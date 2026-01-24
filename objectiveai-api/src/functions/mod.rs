//! Function operations.
//!
//! Functions are composable scoring pipelines that combine vector completions
//! and other functions to produce scores.

mod client;
/// Function execution client and types.
pub mod executions;
mod flat_task_profile;
/// Fetcher for Function definitions from GitHub.
pub mod function_fetcher;
/// Fetcher for Profile definitions from GitHub.
pub mod profile_fetcher;
/// Profile operations.
pub mod profiles;
/// Client for listing functions and getting usage statistics.
pub mod retrieval_client;

pub use client::*;
pub use flat_task_profile::*;
