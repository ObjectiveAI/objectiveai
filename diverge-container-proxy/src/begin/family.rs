//! Which begin the connection had.

/// The family the begin scope speaks: the two carry the same twelve
/// asks in the same order, each in its own frame, and a proxy that
/// asks has to know which.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// An agent container's begin.
    Agents,
    /// A tool container's begin.
    Tools,
}
