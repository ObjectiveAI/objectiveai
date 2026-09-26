//! The tools a container declares, run by the caller.

use std::future::Future;

use crate::shared::containers::tools::Tool;
use crate::shared::error::Error;

/// The caller's answer to a provider asking it to deploy the tools a
/// container declared at registration.
///
/// Each is a tool container the caller runs — a
/// `containers::tools::run` from the tool's image, limits and
/// arguments, with whatever mounts the caller chooses — and serves to
/// the container under the tool's name among the MCP servers it
/// answers the container's tool calls with. The provider asks once
/// per run, before the id, and only when the container declared at
/// least one; the run goes on to its id on `Ok` and ends on `Err`,
/// the container stopped, with the error carried to the caller as the
/// run's own. Whether a tool's own tools are deployed in turn is the
/// caller's policy.
pub trait ToolDeployer: Send + Sync {
    /// Run every tool, or say why not.
    fn deploy(&self, tools: Vec<Tool>) -> impl Future<Output = Result<(), Error>> + Send;
}
