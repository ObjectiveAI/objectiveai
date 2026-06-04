//! Async handlers for every cli leaf command (except `api`,
//! `schemas`, the internal `instance` subprocess runner, and
//! clap's `external` plugin dispatch). Stubs today; typed `Args`
//! + `pub async fn handle(...)` signatures land in follow-up
//! commits.

mod command;
pub use command::*;

mod command_request;
pub use command_request::*;

#[cfg(feature = "mcp")]
mod command_response;
#[cfg(feature = "mcp")]
pub use command_response::*;

mod from_args_error;
pub use from_args_error::*;

#[cfg(feature = "cli-executor")]
mod command_executor;
#[cfg(feature = "cli-executor")]
pub use command_executor::*;

#[cfg(feature = "cli-executor")]
mod stream_once;
#[cfg(feature = "cli-executor")]
pub use stream_once::*;

mod ok;
pub use ok::*;

mod path_ref;
pub use path_ref::*;

mod response_schema;
pub use response_schema::*;

pub mod agents;
pub mod config;
pub mod functions;
pub mod logs;
pub mod mcp;
pub mod plugins;
pub mod swarms;
pub mod tools;
pub mod update;
pub mod viewer;
