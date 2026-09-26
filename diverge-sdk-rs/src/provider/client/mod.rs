//! The caller half of the provider protocol: what a caller supplies.
//!
//! Requests, and answers. A caller opens scopes and channels through
//! the frame-level [`client`](crate::wire::client) — every endpoint's
//! `client::execute` performs one exchange rather than describing it
//! — and a provider opens channels back into it for the things it
//! cannot reach itself, which the caller answers through the traits
//! here. Every endpoint has its executor: eleven of the `volumes`,
//! `images::check` and `version` collapse into a call; the three
//! `containers` scopes hand back a handle that holds the container's
//! life.
//!
//! # Answering what the far end asks
//!
//! While a container runs, its provider opens channels into the
//! caller for what lives with the caller, and a caller supplies one
//! answerer per kind, assembled in an [`Answerers`]:
//!
//! | trait | answers |
//! |-------|---------|
//! | [`OciStore`] | whether the caller holds an image, and its manifest and blobs |
//! | [`ConnectionAuthorizer`] | whether a connector may attach |
//! | [`ToolDeployer`] | the tool containers the container declared, run |
//! | [`PostgresDialer`] | the container's database connections |
//! | [`CommandRunner`] | the commands the container asks run |
//! | [`Vault`] | the container's secrets, with locks |
//! | [`McpServer`] | the container's tool calls outward |
//! | [`FuseServer`] | the files and directories mounted live |
//!
//! Each answers in the wire's own vocabulary — an absence is the
//! empty finish, a refusal the frame that says so — so none carries
//! an error type of its own. The executors under
//! [`containers`](crate::provider::endpoints::containers) read every
//! ask off the scope, decode it, and answer it on a task of its own
//! through these; the connect scope, which is asked for nothing but
//! the content of its own writes, takes none of them.

mod answerers;
mod command_runner;
mod connection_authorizer;
mod fuse_server;
mod mcp_server;
mod oci_store;
mod postgres_dialer;
mod tool_deployer;
mod vault;

pub use answerers::*;
pub use command_runner::*;
pub use connection_authorizer::*;
pub use fuse_server::*;
pub use mcp_server::*;
pub use oci_store::*;
pub use postgres_dialer::*;
pub use tool_deployer::*;
pub use vault::*;
