//! Everything a caller answers a running container with, in one
//! value.

use std::sync::Arc;

/// The seven things a provider opens channels back into a caller for
/// while a container runs, assembled once.
///
/// A run executor takes one of these and answers every server-opened
/// channel through it: the image's pieces from [`oci`](Self::oci), the
/// dependencies the container declared from
/// [`dependencies`](Self::dependencies), the
/// container's database connections through [`postgres`](Self::postgres),
/// its daemon connection through [`daemon`](Self::daemon), its secrets
/// through [`vault`](Self::vault), its tool calls through
/// [`mcp`](Self::mcp), and the files it mounted live through
/// [`fuse`](Self::fuse). Each is an [`Arc`], because every ask is
/// answered on a task of its own — answers may be given in any order,
/// and an agent making several calls at once is the ordinary case —
/// and the seven are cloned into as many tasks as there are asks.
///
/// The type parameters are the seven traits of this module, bound
/// where an executor takes the value; the struct itself binds nothing,
/// so it can be built and moved around without naming them. A caller
/// that serves nothing of a kind implements that trait as the empty
/// answer: `None`, a denial, an empty stream, an error.
#[derive(Debug)]
pub struct Answerers<O, T, P, D, V, M, F> {
    /// Whether the caller holds an image, and under what name, and its
    /// manifests and blobs
    /// ([`OciStore`](super::OciStore)).
    pub oci: Arc<O>,
    /// The dependencies the container declared, run
    /// ([`DependencyDeployer`](super::DependencyDeployer)).
    pub dependencies: Arc<T>,
    /// The database the container dials
    /// ([`PostgresDialer`](super::PostgresDialer)).
    pub postgres: Arc<P>,
    /// The frames of the container's daemon connection
    /// ([`Daemon`](super::Daemon)).
    pub daemon: Arc<D>,
    /// The container's secrets ([`Vault`](super::Vault)).
    pub vault: Arc<V>,
    /// The MCP servers the container calls
    /// ([`McpServer`](super::McpServer)).
    pub mcp: Arc<M>,
    /// The files and directories mounted live
    /// ([`FuseServer`](super::FuseServer)).
    pub fuse: Arc<F>,
}

impl<O, T, P, D, V, M, F> Clone for Answerers<O, T, P, D, V, M, F> {
    fn clone(&self) -> Self {
        Answerers {
            oci: Arc::clone(&self.oci),
            dependencies: Arc::clone(&self.dependencies),
            postgres: Arc::clone(&self.postgres),
            daemon: Arc::clone(&self.daemon),
            vault: Arc::clone(&self.vault),
            mcp: Arc::clone(&self.mcp),
            fuse: Arc::clone(&self.fuse),
        }
    }
}
