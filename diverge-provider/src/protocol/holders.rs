//! Who holds a volume.

/// What a [`Volume`](super::volume::Volume) answers about its hold,
/// as of now: nobody; the running containers and the FUSE serves
/// that have it, counted apart; or the one stat, read, write,
/// filetree, edit or delete that has it to itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Holders {
    /// Nothing holds it: a shared hold or the exclusive one would be
    /// taken, as the rule allows.
    Free,
    /// The shared holds, by kind. Never both `0`: that is
    /// [`Free`](Self::Free).
    Shared {
        /// Running containers that have it mounted — a request that
        /// named the volume twice counting once — and image builds
        /// reading it as their context.
        runs: u32,
        /// `volumes::fuse_serve` scopes that hold it.
        fuse_serves: u32,
    },
    /// The exclusive hold: a stat, a read, a write, a filetree, an
    /// edit or a delete in flight.
    Locked,
}
