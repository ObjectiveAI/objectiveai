//! What a container declared at registration, as the provider relays
//! it.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use serde_json::Error;

use super::super::super::dependencies::Template;
use super::super::super::secrets::Declaration;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// The dependencies and the secrets' declaration, as the container's
/// proxy answered `Begun` with them, and the run's id.
///
/// Borrowed where the provider encodes it — both are the `Begun`'s
/// for the run's life, see the agents begin's
/// [`Registered`](crate::container_proxy::outside::endpoints::agents::begin::server::response::Registered)
/// and the tools begin's
/// [`Registered`](crate::container_proxy::outside::endpoints::tools::begin::server::response::Registered)
/// — and owned where the caller decodes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request<'a> {
    /// The run's id: the one the `Id` response carries after the
    /// registration, told here first so that the caller can serve the
    /// container's own paths into the dependencies — a
    /// `containers::fuse_serve` names a container by it — before the
    /// id is answered. The provider has entered the container among
    /// those it runs by the time it asks.
    pub id: Cow<'a, str>,
    /// The dependencies, in the order the program declared them.
    /// Absent when empty, which a tool container's always is.
    #[serde(default, skip_serializing_if = "no_dependencies")]
    pub dependencies: Cow<'a, [Template]>,
    /// The secrets the program takes, each by name to its mode, in the
    /// order the program declared them: see
    /// [`Declaration`]. Absent when empty.
    #[serde(default, skip_serializing_if = "no_secrets")]
    pub secrets: Cow<'a, Declaration>,
}

/// Whether the list is empty, for serde.
fn no_dependencies(dependencies: &Cow<'_, [Template]>) -> bool {
    dependencies.is_empty()
}

/// Whether the declaration is empty, for serde.
fn no_secrets(secrets: &Cow<'_, Declaration>) -> bool {
    secrets.is_empty()
}

/// Its JSON, and nothing in front of it. The tag that says which
/// request this is belongs to whichever frame carries it.
impl Encode for Request<'_> {
    /// The ordinary JSON failure.
    type Error = Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Error> {
        serde_json::to_writer(out, self)
    }
}

impl Decode<'_> for Request<'_> {
    /// The ordinary JSON failure.
    type Error = Error;

    fn decode(bytes: &[u8]) -> Result<Self, Error> {
        serde_json::from_slice(bytes)
    }
}
