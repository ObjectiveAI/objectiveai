//! What a container declares at registration, served by the caller.

use std::future::Future;

use crate::shared::containers::dependencies::Template;
use crate::shared::containers::secrets::{Declaration, Values};
use crate::shared::error::Error;
use crate::shared::permission::Grant;

/// The caller's answer to a provider asking it to register a
/// container: deploy the dependencies its program declared, resolve
/// the secrets it takes, and hold it to the permissions it declared.
///
/// Each dependency is a tool container the caller runs — a
/// `containers::tools::run` from the template's image, limits and
/// arguments, the agent's own paths served into it as the template's
/// mounts say, on the database and under the grants it names — and
/// serves to the container under the template's id among the MCP
/// servers it answers the container's tool calls with. Each secret
/// is a name the container's template declares, bound by the
/// caller's record to a vault key: the caller checks the name is one
/// the template declares, locks a free value of the key for a
/// `rotating` one and reads one for a `fixed` one, and answers every
/// name with its value — the container's first feed, which the
/// provider hands it before the id goes out. Each permission is a
/// grant the container's program holds over the caller for the run, a
/// tool's alone, and the whole of what it may ask. The provider asks
/// once per run, before the id, and only when the container declared
/// at least one of any; the run goes on to its id on `Ok` and ends
/// on `Err`, the container stopped, with the error carried to the
/// caller as the caller's policy and never a value in it.
pub trait Registrar: Send + Sync {
    /// Run every dependency for the container running under `id` on
    /// the provider that asks — the id its run will answer, told
    /// first so the caller can serve the container's paths into the
    /// dependencies — resolve every secret in `secrets` through the
    /// container's binding, hold the container to `permissions`, and
    /// answer the secrets' first values; or say why not.
    fn register(
        &self,
        id: String,
        dependencies: Vec<Template>,
        secrets: Declaration,
        permissions: Vec<Grant>,
    ) -> impl Future<Output = Result<Values, Error>> + Send;
}
