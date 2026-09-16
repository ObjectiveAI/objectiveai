//! Everything a caller answers a running container with, in one
//! value.

use std::sync::Arc;

/// The seven things a provider opens channels back into a caller for
/// while a container runs, assembled once.
///
/// A run executor takes one of these and answers every server-opened
/// channel through it: the image's pieces from [`oci`](Self::oci), a
/// connector's admission from [`authorizer`](Self::authorizer), the
/// container's database connections through [`postgres`](Self::postgres),
/// its commands through [`commands`](Self::commands), its secrets
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
pub struct Answerers<O, A, P, C, V, M, F> {
    /// The manifests and blobs of images the caller holds
    /// ([`OciStore`](super::OciStore)).
    pub oci: Arc<O>,
    /// Whether a connector may attach
    /// ([`ConnectionAuthorizer`](super::ConnectionAuthorizer)).
    pub authorizer: Arc<A>,
    /// The database the container dials
    /// ([`PostgresDialer`](super::PostgresDialer)).
    pub postgres: Arc<P>,
    /// The commands the container asks run
    /// ([`CommandRunner`](super::CommandRunner)).
    pub commands: Arc<C>,
    /// The container's secrets ([`Vault`](super::Vault)).
    pub vault: Arc<V>,
    /// The MCP servers the container calls
    /// ([`McpServer`](super::McpServer)).
    pub mcp: Arc<M>,
    /// The files and directories mounted live
    /// ([`FuseServer`](super::FuseServer)).
    pub fuse: Arc<F>,
}

impl<O, A, P, C, V, M, F> Clone for Answerers<O, A, P, C, V, M, F> {
    fn clone(&self) -> Self {
        Answerers {
            oci: Arc::clone(&self.oci),
            authorizer: Arc::clone(&self.authorizer),
            postgres: Arc::clone(&self.postgres),
            commands: Arc::clone(&self.commands),
            vault: Arc::clone(&self.vault),
            mcp: Arc::clone(&self.mcp),
            fuse: Arc::clone(&self.fuse),
        }
    }
}
