//! Start, or stop: what `diverge daemon` and `diverge provider` each
//! take beside `config`.

use clap::Subcommand;

/// The two things done to a program on this host: see
/// [`lifecycle`](crate::lifecycle) for each in full. The program is
/// the daemon's or the provider's own binary beside this one, run on
/// the CLI's root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum Lifecycle {
    /// Start the program on the root, detached, its output in
    /// `<root>/daemon.log` or `<root>/provider.log`; the command ends
    /// once the program holds the root's lock, and fails if one
    /// already does or the program dies before it has.
    Start,
    /// Stop the program running on the root, asked over its own port
    /// on this host; the command ends once the root's lock is free,
    /// which is the whole of the program's teardown done.
    Stop,
}
