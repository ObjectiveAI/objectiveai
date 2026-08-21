//! Why a write's content stopped, from either place it can come from.

use std::fmt;

use crate::shared::error::Error;

/// The content for a write ran out early, and this is whose fault it
/// was.
///
/// What an item of the stream
/// [`Container::write`](super::container::Container::write) consumes
/// fails with. It is an enum because a write's content has two origins
/// and they fail in incompatible vocabularies.
///
/// # The two origins
///
/// A caller's write sends its content over the wire, as responses on a
/// channel the provider opened. When that stops early the frame says so
/// with a [`shared::error::Error`](crate::shared::error::Error) — one
/// JSON value, from somebody else's process, meaning whatever that
/// caller meant. That is [`Wire`](Self::Wire).
///
/// A [`transfer`](crate::shared::container::transfer) has no wire in
/// it. Both containers are the provider's, so the bytes go from a
/// [`read`](super::container::Container::read) on one straight into a
/// write on the other without ever becoming frames — which is the whole
/// point of having a transfer rather than a read piped through a
/// caller. A read fails with the provider's own error, and that is
/// [`Container`](Self::Container).
///
/// # Why not flatten them
///
/// Making everything a
/// [`shared::error::Error`](crate::shared::error::Error) would mean a
/// provider converting its own read failure into opaque JSON so it
/// could hand it to its own write, in the same process, for nobody's
/// benefit. That is the mistake
/// [`ContainerDeployer::Error`](super::container_deployer::ContainerDeployer::Error)
/// exists to avoid: the type that knows most about what happened, lost
/// at the one moment it is in hand.
///
/// Making everything the provider's own error is not available. A
/// caller's failure is a JSON value from another process, and nothing
/// turns one of those into a runtime's error type.
///
/// So neither collapses into the other, and the type says so.
///
/// # It is generic rather than tied to the trait
///
/// [`Container::write`](super::container::Container::write) uses it as
/// `ContentError<Self::Error>`, but nothing here names that. It is a
/// plain two-armed enum over "the wire" and "something else", which is
/// what lets a provider hold one before it has decided which container
/// it is about to write into.
#[derive(Debug, Clone, PartialEq)]
pub enum ContentError<E> {
    /// A caller's content stopped.
    ///
    /// Relayed verbatim from the channel it arrived on, meaning
    /// whatever the caller meant by it. A provider does not read it and
    /// could not usefully — see
    /// [`shared::error::Error`](crate::shared::error::Error) for why it
    /// says so little.
    ///
    /// It is not a failure of the container being written into. The
    /// write is abandoned because there is nothing left to write, not
    /// because anything here went wrong.
    Wire(Error),
    /// Another container's read stopped.
    ///
    /// The provider's own error, from the
    /// [`read`](super::container::Container::read) feeding this write.
    /// Which happens for a
    /// [`transfer`](crate::shared::container::transfer), where the
    /// source is a container rather than a caller.
    ///
    /// Whether it is a refused read or a truncated one is not
    /// something a read distinguishes, so this does not either.
    Container(E),
}

impl<E> fmt::Display for ContentError<E>
where
    E: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContentError::Wire(_) => {
                f.write_str("the caller's content stopped")
            }
            ContentError::Container(error) => {
                write!(f, "the source container's read stopped: {error}")
            }
        }
    }
}

impl<E> std::error::Error for ContentError<E>
where
    E: std::error::Error + 'static,
{
    /// [`Wire`](ContentError::Wire) has no source, because what it
    /// carries is not a Rust error and deliberately does not implement
    /// one — see
    /// [`shared::error::Error`](crate::shared::error::Error). A caller
    /// that wants what is inside it matches the variant.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ContentError::Container(error) => Some(error),
            ContentError::Wire(_) => None,
        }
    }
}
