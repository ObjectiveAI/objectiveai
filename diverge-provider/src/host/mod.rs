//! What this provider supplies from the host it runs on.
//!
//! [`protocol`](crate::protocol) holds the socket's scopes, reads every
//! request, and serves every endpoint; what it cannot supply is what a
//! provider IS — a place to run containers, a store of volumes, a
//! registry, and a judgment about who may connect — and this module
//! supplies those, as the protocol's six provider-side traits:
//!
//! | trait | supplies |
//! |-------|----------|
//! | `ContainerDeployer` | a container, running, with the proxy inside and port 14979 reachable |
//! | `Container` | the address of that proxy, and the stop |
//! | `VolumeManager` | the volumes of every identity |
//! | `ImageRegistry` | the registry a runtime pulls from, for an image taken from the caller |
//! | `ImageChecker` | whether an image can be had without the caller |
//! | `UnbrokeredAuthorizer` | the identity a credential establishes |
//!
//! This is ONE provider, not the only one. The specification binds any
//! provider; this module is an implementation that conforms to it, on
//! one host with podman, and another may take [`protocol`](crate::protocol)
//! and supply its own.
//!
//! Here is where the implementation goes; [`container_deployer`], [`volume_manager`],
//! [`image_checker`], [`image_registry`] and
//! [`unbrokered_authorizer`] supply the six, every method of each
//! written. [`tools`] is every program the provider runs — podman,
//! and e2fsprogs, `mount` and `ssh` through it or beside it — and
//! the one way it runs them. [`watch`] is a directory of this host
//! walked and watched, the filetree of a fixed volume, one module
//! for every host through `notify`. What the provider is told is the
//! `provider` block of the one `config.yaml`,
//! [`diverge_sdk::config::provider`], read from the root the SDK
//! finds; the provider keeps its state under `<root>/provider/`.
//! [`serve`] is the provider running: the pieces built once, a
//! WebSocket accepted on its port or dialled to each peer it is told
//! of, every connection handed to the protocol's dispatch, and the
//! stop.

mod limit;

pub use limit::*;

pub mod container_deployer;
pub mod hook;
pub mod image_checker;
pub mod image_registry;
pub mod serve;
pub mod tools;
pub mod unbrokered_authorizer;
pub mod volume_manager;
pub mod watch;
