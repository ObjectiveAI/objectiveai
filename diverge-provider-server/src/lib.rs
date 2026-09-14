//! The reference provider of the Diverge provider protocol.
//!
//! A provider is a server of the protocol that
//! [`diverge-provider-sdk`](https://docs.rs/diverge-provider-sdk)
//! defines and `https://provider.diverge.network` specifies. The SDK
//! holds the socket, reads every request, and serves every endpoint;
//! what it cannot supply is what a provider IS — a place to run
//! containers, a store of volumes, a store of content, a registry, and
//! a judgment about who may connect — and this crate supplies those,
//! as the SDK's six provider-side traits:
//!
//! | trait | supplies |
//! |-------|----------|
//! | `ContainerDeployer` | a container, running, with the proxy inside and port 14979 reachable |
//! | `Container` | the address of that proxy, and the stop |
//! | `VolumeManager` | the volumes of every identity, and their watches |
//! | `ContentStore` | mounted content held by identity, verified |
//! | `ImageRegistry` | the registry a runtime pulls a caller-held image from |
//! | `ImageChecker` | whether a `server` image can be served |
//! | `UnbrokeredAuthorizer` | the identity a credential establishes |
//!
//! This is ONE provider, not the only one. The specification binds any
//! provider; this crate is an implementation that conforms to it, and
//! a library so that its pieces can be taken separately by another.
//!
//! The crate is the place the implementation goes, named and in the
//! workspace; [`volume_manager`] is the first trait supplied, and its
//! methods are not written yet.

pub mod config;
pub mod hook;
pub mod volume_manager;
