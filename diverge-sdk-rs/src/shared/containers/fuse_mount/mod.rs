//! One more file or directory the caller serves into a running
//! container.
//!
//! A client opens a channel on a run's scope with a
//! [`request::Request`] — where in the container, under what id, and
//! whether a file or a directory — and the provider answers on that
//! same channel with a [`response::Frame`] once the proxy has made
//! the mount, or with why it is not made. The mount is what a
//! [`FuseMount`](super::request::FuseMount) of the run's request is,
//! made after the id instead of before it: the same asks ride the
//! same channels, keyed by the id, and the mount lives until the run
//! ends. Nothing unmounts one.
//!
//! # What the provider refuses on its own account
//!
//! A path that is a mount's the container holds, lies inside one or
//! holds one — a volume mount's or a FUSE mount's, the request's or
//! one made since — is `Overlaps`; an id the container holds a FUSE
//! mount under is `Taken`. Each is answered without asking the
//! proxy. What the proxy refuses is `Error`, in the proxy's own
//! words.

pub mod request;
pub mod response;
