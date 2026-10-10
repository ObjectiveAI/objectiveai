//! The reference provider of the Diverge provider protocol, in two
//! halves.
//!
//! A provider is a server of the protocol that
//! [`diverge-sdk`](https://docs.rs/diverge-sdk) defines and
//! `https://provider.diverge.network` specifies. The SDK is the wire,
//! the frames and the clients; the server half of the provider
//! protocol is here, as two root modules and nothing else:
//!
//! | module | is |
//! |--------|----|
//! | [`protocol`] | the server half for ANY provider: the dispatch over every request, one handler per endpoint, the machinery of a container's run, and the traits a provider implements — a deployer, a volume manager, an image checker, a registry, a container — with the registries every connection shares |
//! | [`host`] | what THIS provider supplies, from the host it runs on: podman for the containers, ext4 images on disk for the volumes, a loopback registry, the hooks, the configuration's judgment of a credential, and the listener that hands every connection to [`protocol`] |
//!
//! Another provider imports [`protocol`], implements its traits, and
//! calls its [`handle`](protocol::handle::handle) per connection; the
//! binary of this crate does exactly that with [`host`].

pub mod host;
pub mod protocol;
