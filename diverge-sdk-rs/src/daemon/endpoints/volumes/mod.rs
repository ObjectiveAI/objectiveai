//! Volumes: the directories providers hold, reached through the daemon.
//!
//! A volume is a provider's: a directory the provider offers under a
//! name, as the provider protocol's
//! [`volumes`](crate::provider::endpoints::volumes) states, kept on
//! that provider's disk, mounted into containers that run there, and
//! served over FUSE into containers that run elsewhere. The daemon
//! reaches every one of them on every provider it knows, and names each
//! by its [`reference::Volume`](crate::daemon::reference::Volume): the
//! provider's identity and the name. [`create`] asks a provider for a
//! volume; [`get`] answers one as a list would; [`list`] lists them
//! across providers, narrowed; [`delete`] destroys one nothing uses;
//! [`edit`] changes how much one reserves, its mode, or both; [`stat`]
//! walks one for how much of it is used and the hash of its content;
//! [`filetree`] answers one's tree, once; [`download`] sends the client
//! a file or a directory out of one, [`upload`] puts files into one,
//! and [`transfer`] copies out of one into an agent, a tool or another
//! volume, the bytes never reaching the client. A
//! volume is a [`Destination`](crate::daemon::transfer::Destination) of
//! every other family's transfer too.
//!
//! # At rest, or held
//!
//! A volume is read, written, walked, resized and deleted at rest, as
//! the provider does each: nothing of that happens while a running
//! container has the volume, and no run takes it while that happens.
//! The daemon holds a volume for the length of a download, an upload or
//! a transfer at either end of it, and a request that would take a held
//! volume — one a running container has, or one another download,
//! upload or transfer is on — is answered `Held`, and asked again once
//! the volume is free. Nothing is started for a volume: it is not a
//! container.
//!
//! # In use
//!
//! A delete is refused as `InUse` while any agent or tool of the
//! daemon's names the volume in its mounts — a volume mount of the
//! provider it is pinned to, or a FUSE mount of a file or a directory
//! in it — whether or not that container is running, since deleting it
//! would break the container's next run; and while a download, an
//! upload or a transfer is on it. The provider's own refusal of a
//! mounted volume is the same answer.
//!
//! # What a volume is not
//!
//! A volume carries no tags and no creator: it is the provider's, not
//! the daemon's, and a provider that offers one on its own made it with
//! no account of the daemon's. Naming a volume in a container's mounts,
//! at the container's create or edit, takes the `mount` grant over it:
//! see [`grant`](crate::daemon::grant).

pub mod create;
pub mod delete;
pub mod download;
pub mod edit;
pub mod filetree;
pub mod get;
pub mod list;
pub mod stat;
pub mod transfer;
pub mod upload;
