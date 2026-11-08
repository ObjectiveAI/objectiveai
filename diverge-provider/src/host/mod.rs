//! What this provider supplies from the host it runs on.
//!
//! [`protocol`](crate::protocol) holds the socket's scopes, reads every
//! request, and serves every endpoint; what it cannot supply is what a
//! provider IS — a place to run containers, a store of volumes, a
//! registry, and a judgment about who may connect — and this module
//! supplies those, as the protocol's seven provider-side traits, the
//! handshake's [`Authenticator`](diverge_sdk::wire::handshake::Authenticator),
//! and the registry its runtime pulls from:
//!
//! | trait | supplies |
//! |-------|----------|
//! | `ContainerDeployer` | a container, running, with the proxy inside and port 14979 reachable |
//! | `Container` | the address of that proxy, and the stop |
//! | `VolumeManager` | the volumes of every identity |
//! | `ImageHolder` | whether an image is held for a caller — its Store, or a registry the provider uses — and never the cache |
//! | `ImageManager` | the Store: the images every identity owns on this provider, by digest, one copy of each blob for every owner |
//! | `OciStore` | the image a caller's pull names, served from its Store or fetched through from a registry |
//! | `CapacityReporter` | how much a caller may still use: the stores' room and what remains of the two caps |
//! | `Authenticator` | the provider's token at an issuer, and whether a peer's token is one it serves |
//!
//! This is ONE provider, not the only one. The specification binds any
//! provider; this module is an implementation that conforms to it, on
//! one host with podman, and another may take [`protocol`](crate::protocol)
//! and supply its own.
//!
//! Here is where the implementation goes; [`container_deployer`], [`volume_manager`],
//! [`image_manager`] — the `ImageHolder` and the `OciStore` as well as
//! the `ImageManager`, the three answering from one resolution —
//! [`capacity_reporter`] and [`authenticator`] supply the eight, every
//! method of each written. [`image_registry`] is the registry podman
//! pulls from, one random token per pull, from the Store or read
//! through to a run's caller; [`registry_client`] the minimal OCI
//! Distribution client a push, a `has`, a pull and a hash fetch from
//! a registry with. [`tools`] is every program the provider runs — podman,
//! and e2fsprogs, `mount` and `ssh` through it or beside it — and
//! the one way it runs them. [`watch`] is a directory of this host
//! walked and watched, the filetree of a fixed volume, one module
//! for every host through `notify`. What the provider is told is the
//! one `provider.yaml`, [`crate::config`], read from the root the
//! provider finds; the provider keeps its state under
//! `<root>/provider/`.
//! [`serve`] is the provider running: the pieces built once, a
//! WebSocket accepted on its port or dialled to each peer it is told
//! of, every connection handed to the protocol's dispatch, and the
//! stop.

mod limit;

pub use limit::*;

pub mod authenticator;
pub mod capacity_reporter;
pub mod container_deployer;
pub mod hook;
pub mod image_manager;
pub mod image_registry;
pub mod registry_client;
pub mod serve;
pub mod tools;
pub mod volume_manager;
pub mod watch;
