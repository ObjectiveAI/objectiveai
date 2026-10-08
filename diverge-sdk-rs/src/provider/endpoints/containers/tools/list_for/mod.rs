//! Finding the tool containers somebody else runs, each with its
//! runner's leave.
//!
//! Split by who SENDS: [`client`] is the caller's traffic, [`server`]
//! the provider's.
//!
//! A provider's directory of containers is never read whole: a
//! container is reached by its id, and nothing hands ids out. This is
//! the one way to learn them. A client names an identity — somebody
//! else's, which is why this is a listing FOR — and the provider asks
//! every tool container that identity runs, on its run scope, whether
//! the lister may see it — the same yes or no a runner gives a
//! connector — and sends each container the moment its runner says
//! yes, one response each, in whatever order the answers come. A
//! runner that says no, or does not answer, keeps its container
//! unlisted. Once every runner asked at the opening has answered or
//! gone, the provider says the listing is whole, and keeps the scope
//! open: a tool container the identity starts after is asked about
//! the same way and sent as added when its runner allows, and one
//! whose run ends is sent as removed to every lister it was sent to.
//! The scope has no end of its own — an identity that runs nothing
//! is still watched — and ends at the lister's stop, the one channel
//! a lister opens on it, at the lister's connection ending, or at
//! the provider's error. Agent containers are not asked about and
//! never listed: an agent container takes no connector, so there is
//! nothing a lister could do with its id.
//!
//! What a listing does NOT do is join anything. A container listed
//! is a container named: the id is what a [`connect`](super::connect)
//! then offers, with an authorization, and the runner is asked
//! again.

pub mod client;
pub mod server;
