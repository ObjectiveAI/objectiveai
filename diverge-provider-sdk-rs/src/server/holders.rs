//! Who holds a volume.

/// What a [`Volume`](super::volume::Volume) answers about its hold,
/// as of now: nobody, the containers mounting it, or the one stat,
/// read, write, filetree, edit or delete that has it to itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Holders {
    /// Nothing holds it: a mount or a lock would be taken.
    Free,
    /// This many shared holds — containers that have it mounted,
    /// counting a request that named it twice twice.
    Mounted(u32),
    /// The exclusive hold: a stat, a read, a write, a filetree, an
    /// edit or a delete in flight.
    Locked,
}
