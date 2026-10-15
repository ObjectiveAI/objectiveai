//! The dependencies a container declares, run by the caller.

use std::future::Future;

use crate::shared::containers::dependencies::Template;
use crate::shared::error::Error;

/// The caller's answer to a provider asking it to deploy the
/// dependencies a container declared at registration.
///
/// Each is a tool container the caller runs — a
/// `containers::tools::run` from the template's image, limits and
/// arguments, the agent's own paths served into it as the template's
/// mounts say, on the database and under the grants it names — and
/// serves to the container under the template's id among the MCP
/// servers it answers the container's tool calls with. The provider asks once
/// per run, before the id, and only when the container declared at
/// least one; the run goes on to its id on `Ok` and ends on `Err`,
/// the container stopped, with the error carried to the caller as the
/// caller's policy.
pub trait DependencyDeployer: Send + Sync {
    /// Run every dependency for the container running under `id` on
    /// the provider that asks — the id its run will answer, told
    /// first so the caller can serve the container's paths into the
    /// dependencies — or say why not.
    fn deploy(&self, id: String, dependencies: Vec<Template>) -> impl Future<Output = Result<(), Error>> + Send;
}
