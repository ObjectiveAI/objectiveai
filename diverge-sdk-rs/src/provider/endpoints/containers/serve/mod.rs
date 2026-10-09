//! Serving a running container: the FUSE asks, answered from it.
//!
//! A caller names a container it is running, by id, and a subtree of
//! it by path, and holds the scope; for as long as it does, it opens
//! channels on the scope carrying the nine asks of
//! [`fuse`](crate::shared::containers::fuse) — a stat, a piece read, a
//! piece written, a truncate, a setattr, a listing, a removal, a
//! rename, a mkdir — with no mount id, since the scope is the subtree,
//! and the provider answers each from the container's own filesystem
//! as a caller's server would answer a mount. Split by who SENDS, as
//! everywhere else: the ask and the stop are in [`client`], the
//! answers in [`server`].
//!
//! # What it is for
//!
//! A [`volumes::serve`](crate::provider::endpoints::volumes::serve) is a
//! volume at the far end of a FUSE bridge: a mount on one provider
//! answered from storage on another. This is the same bridge with a
//! running container at the far end — an agent's working directory
//! mounted into a tool's container, a tool's output mounted into an
//! agent's — so that two containers, on one provider or two, share
//! a directory live, and the caller that runs the container is the
//! one in the middle. The vocabulary is the volume serve's exactly,
//! so the bridge is mechanical.
//!
//! # The scope is the subtree
//!
//! The request names a path, as components from the container's
//! root, and every ask's path is relative to it: the empty path is
//! the subtree's root, which is the directory the request named. A
//! path that names the root of the container serves the container
//! whole, less what the proxy leaves out of every tree — `/proc`,
//! `/sys` and `/dev`, under which nothing is answered. A path that
//! is not a directory of the container, or that lies under one of
//! those three, is the serve refused.
//!
//! # Every ask passes through
//!
//! A container has no mode: there is no layer and no read-only
//! answer. Every mutation the provider answers `ok` is in the
//! container, made by the proxy beside the program, and the program
//! sees it as it sees any other change to its own filesystem. The
//! asks are relayed to the proxy as they arrive and the answers back
//! as they come; the provider buffers nothing and reads nothing.
//!
//! # Its runner only
//!
//! Only the identity running the container may serve it: a connector
//! attached to a tool container may not, and nobody may serve an
//! agent container but the identity that opened its run. An id under
//! which nothing runs, and an id whose runner the caller is not, are
//! refused alike, and indistinguishably: whether an id exists is not
//! told to a caller that may not reach it.
//!
//! # The tree, as the container holds it
//!
//! A caller that holds the scope may open a
//! [`filetree`](client::channel_request::Frame::Filetree) channel on
//! it, any number of times, and the provider answers each with a
//! [`filetree`](crate::shared::filetree) stream of the subtree AS THE
//! CONTAINER HOLDS IT: one snapshot, then one frame per change, for
//! as long as the scope lives, and the finish with the scope. The
//! changes are every change in the subtree — the serve's own asks,
//! and whatever the program running in the container does — because
//! the proxy watches the subtree as it watches the container's tree
//! for a run's filetree channel.
//!
//! # How it ends
//!
//! The caller's stop ends the scope, and so does the caller's
//! connection ending, and so does the container ending — its run
//! stopped, or the container gone on its own. Nothing is released
//! at the end: the container is its runner's, and a serve held
//! nothing of it.

pub mod client;
pub mod server;
