//! The members an agent's create and a tool's create share.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents::create::client::request::{FuseMount, Provider};

/// Everything a container is made of that its template does not say and
/// its name is not: the template, the account it runs under, the
/// provider pin and its volumes, the FUSE mounts. The deployer is an
/// agent's alone, on its own create: a tool has no dependencies.
/// Flattened into an
/// [agent's](crate::daemon::endpoints::agents::create) and a
/// [tool's](crate::daemon::endpoints::tools::create) create, so its
/// members are the request's own.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Inner {
    /// The template the container is made from, by its id — the hash
    /// its family's `templates::create` answered. An id no template of
    /// the caller's has, or one of the other family, is the create's
    /// error. The template's image, limits and arguments are
    /// the container's for its life.
    pub template: String,
    /// The account the container runs under, if any — a NAMED
    /// [account](crate::daemon::endpoints::accounts), by its name: the
    /// identity it acts as toward the daemon, and what the daemon
    /// judges everything it asks of the daemon by, through the roles
    /// the account holds. One the daemon does not have is the create's
    /// `NoAccount`; one the caller holds no `assign` grant over is its
    /// `Forbidden`; in either case nothing is made. Absent, the
    /// container runs under no account and holds no grant: everything
    /// it asks of the daemon is answered `Forbidden`. The account is
    /// the container's for its life unless an edit replaces or deletes
    /// it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    /// The one provider the container runs on, and the volumes of that
    /// provider made visible inside it. See [`Provider`].
    ///
    /// Absent, the container runs on whichever provider the daemon
    /// chooses, and mounts no volume: a volume is a provider's own
    /// and does not carry across, so a container with state on a
    /// provider's disk is a container of that provider. The
    /// container's for its life, and never the template's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<Provider>,
    /// Files of providers' volumes served LIVE across the daemon,
    /// mounted one each over FUSE.
    ///
    /// Each names a file by a provider, a volume of that provider's
    /// and a path in it, and its path in the container — see
    /// [`FuseMount`]; each may name a different provider. Every one
    /// is mounted before the container runs, and every read and every
    /// write of a piece inside the container is one ask, forwarded by
    /// the daemon to the volume's provider and served from the
    /// volume's file in place. The file is
    /// overwritten in place only; a program that replaces its file by
    /// rename needs a directory mount. Its container path is no other
    /// mount's, and lies inside none, as every mount's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fuse_file_mounts: Vec<FuseMount>,
    /// Directories of providers' volumes served LIVE across the
    /// daemon, mounted one each over FUSE.
    ///
    /// Each names a directory by a provider, a volume of that
    /// provider's and a path in it, and its path in the container —
    /// see [`FuseMount`]; each may name a different provider. The
    /// whole tree under the volume path is what the container sees:
    /// every listing, stat, read or write of a piece, truncation,
    /// change of attributes, creation, removal and rename inside the
    /// container is one ask, forwarded by the daemon to the volume's
    /// provider and served from the volume's tree in place. No other
    /// mount may lie inside it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fuse_directory_mounts: Vec<FuseMount>,
}
