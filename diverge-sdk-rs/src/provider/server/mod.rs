//! The provider half of the provider protocol: what a provider
//! supplies, and the dispatch that consumes it.
//!
//! Everything in [`endpoints`](crate::provider::endpoints) is a
//! message. This is the part that does something with one. It sits on
//! the frame-level [`server`](crate::wire::server): a
//! [`Session`](crate::wire::server::session::Session) yields the
//! scopes a caller opens with the bytes that opened each, and
//! [`handle`] reads those bytes as the provider's requests and serves
//! every one. A [`Container`](container::Container) is an address and
//! a way to stop it, and every exchange with one is spoken over the
//! one WebSocket [`proxy`] dials to that address, in the frames of
//! [`container_proxy::outside`](crate::container_proxy::outside) —
//! this crate's own wire, with the provider's server as its client,
//! and nothing the provider has to speak. Whatever else the provider
//! does to reach a container is on its side of that line.
//!
//! # Every endpoint is handled, and [`handle`] wires them together
//!
//! This module's [`handle`] is the dispatch in front of the endpoint
//! handlers: it reads each request a [`Session`](crate::wire::server::session::Session)
//! yields, decides which endpoint it belongs to, and spawns that
//! endpoint's handler with the decoded request. One call per
//! connection is a provider's whole loop.
//!
//! Ten of the [`volumes`](crate::provider::endpoints::volumes),
//! [`images::check`](crate::provider::endpoints::images::check) and
//! [`version`](crate::provider::endpoints::version) answer and finish, which is
//! the whole of what those endpoints do. The three
//! [`containers`](crate::provider::endpoints::containers) scopes serve for as
//! long as their containers run: a run brings its container up,
//! answers its id, and then carries the container's asks out and the
//! caller's channels in until it ends; a connect joins a running
//! container on its runner's say-so. Every endpoint is handled.
//!
//! # And what a provider supplies
//!
//! Six traits and one type, which are what this half asks FOR
//! rather than provides. They mirror the ones [`client`](crate::wire::client)
//! supplies: something a provider implements, so that the parts this
//! crate cannot know are somebody else's.
//!
//! [`container_deployer`] is the first — where a container actually
//! runs.
//!
//! It is generic across the three endpoints that put a container
//! somewhere, because they differ in what goes in one and not in how
//! one is deployed. What it is handed is a [`deployment`] and the
//! caller it is for — the same opaque identity a [`mount`] carries,
//! because deploying is the most consequential thing a provider does on
//! somebody's behalf and every question worth asking about it needs to
//! know whose. What it
//! hands back is a [`container`] — whatever that provider holds a
//! running container by, with the two things only the deploy learned:
//! where the proxy it started inside answers, and how to end it.
//!
//! A deployment's [`mount`] is the one piece of it that is not simply
//! copied out of a request. A caller names a volume, and a name is
//! unique within the caller it was listed to — so what a handler puts
//! in a deployment is the name together with whoever it authenticated,
//! which is a fact only this half of the connection has.
//!
//! [`volume_manager`] is the second, and with [`volume`] it is the
//! eleven [`volumes`](crate::provider::endpoints::volumes) endpoints: the
//! directories a provider offers. The manager is the namespace —
//! listed, looked up by name, created, deleted, and asked how much
//! room there is — and the [`volume`] it hands back for a name is
//! the one directory, examined and resized. Two traits rather than
//! one because the verbs on a volume that exists act on it in place,
//! and every one of them takes the volume's lock first: the lock a
//! provider keeps on each volume is how this
//! half keeps a mounted volume from being examined, resized, deleted
//! or mounted twice, and the run handlers take it too, for the life
//! of the container.
//!
//! The manager takes a client identity on every method, which is the
//! same fact a [`mount`] carries — a volume's name is unique within
//! the caller it was listed to, so a name alone asks a question with
//! more than one answer.
//!
//! The manager and the deployer do not know about each other. A
//! mount reaches a deployer as a name and an identity, and finding
//! the directory is the deployer's, the way publishing a port already
//! is.
//!
//! [`image_checker`] is the third, and it is the smallest: would you
//! supply this image, to this caller. It is not a
//! [`container_deployer`] method because a check creates nothing and
//! pulls nothing, and because no is an ANSWER here where it would be an
//! error there.
//!
//! 
//! other trait is asked things on behalf of a caller, and this is the
//! one that decides who the caller IS. It is consumed by [`handle`]
//! itself rather than by any endpoint's handler, because a credential
//! belongs to the connection and not to any scope on it.
//!
//! [`image_registry`] is the fifth: the OCI registry a provider runs
//! on its loopback for an image it takes from the CALLER, fed by
//! digest through an [`image_source`] — the run scope's channels to
//! the caller, which is the one thing only this crate can be. The
//! registry's HTTP is the provider's, for the reason above: this
//! crate serves none. [`caller`] is what a run hands the deployer
//! beside the image's name and digest: whether the caller holds it,
//! asked on the scope, and where the registry serves it.
//!
//! [`directory`] is the type: every container the provider is running,
//! by id, shared across connections — because a connector names a
//! container its runner may have started on another socket, and has
//! to find its run scope to be authorized on, and its address to dial.
//!
//! Nothing implements any of them, and all are consumed:
//! [`volume_manager`] and [`volume`] by the eleven
//! [`volumes`](crate::provider::endpoints::volumes) endpoints' handlers,
//! [`image_checker`] by
//! [`images::check`](crate::provider::endpoints::images::check)'s, and
//! [`container_deployer`], [`image_registry`] and
//! — for the volumes a request mounts, locked for the run — the
//! [`volume_manager`] again by the two run handlers of
//! [`containers`](crate::provider::endpoints::containers)
//! — the scopes that put a container somewhere. A connect handler
//! consumes none of those, because it deploys nothing — the container
//! it serves already exists, and is stopped by whoever ran it — and
//! reads the [`directory`] the run handlers write.
//!
//! [`version`](crate::provider::endpoints::version) has a handler too and asks
//! for nothing at all, its answer being a compile-time constant. It is
//! the one request a provider can serve without supplying anything.
//!
//! [`proxy`] faces the other way: not something a provider supplies,
//! but the one thing this crate dials — the proxy inside a container,
//! at the address a deployer reported. What comes back is the
//! frame-level client's [`Handle`](crate::wire::client::handle::Handle),
//! and every executor under
//! [`container_proxy_endpoints`](crate::container_proxy::outside)
//! takes it and opens the scope it serves, which is how a provider
//! answers what a container asks and asks what it wants to know.
//!
//! And in front of all of them stands [`handle`]: take a
//! [`Session`](crate::wire::server::session::Session), the connection's identity and
//! address, and the capabilities above, and every scope the client
//! opens is read, routed and served in a task of its own until the
//! connection ends. One call per connection is the whole of a
//! provider's loop.

pub mod caller;
pub mod container;
pub mod container_deployer;
pub mod deployment;
pub mod directory;
pub mod handle;
pub mod holders;
pub mod image_checker;
pub mod image_registry;
pub mod image_source;
pub mod mount;
pub mod proxy;
pub mod served;
pub mod volume;
pub mod volume_manager;
