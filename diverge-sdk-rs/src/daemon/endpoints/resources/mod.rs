//! Resources: files and directories the daemon holds for a caller,
//! by their hash, to serve into agents.
//!
//! A resource is content a caller [`upload`]ed once — one file, or
//! one directory of files — that the daemon keeps and serves over
//! FUSE into every agent made from a
//! [template](crate::daemon::endpoints::agents::templates) that
//! mounts it. There is no name: a resource is known by its id, which
//! is its hash, so the same content uploaded twice is one resource
//! and a caller that holds the bytes knows the id without asking.
//! [`list`] names every resource the caller has; [`delete`] removes
//! one no agent mounts.
//!
//! # The id is Go's dirhash
//!
//! A FILE resource's id is the lowercase hexadecimal SHA-256 of its
//! bytes, sixty-four characters: the hash `dirhash` writes for one
//! file on each line of its summary. A DIRECTORY resource's id is
//! `h1:` followed by the standard base64 of the SHA-256 of that
//! summary — its files sorted bytewise by path, one line each of the
//! file's hexadecimal hash, two spaces, the path, a newline — exactly
//! what `HashDir(root, "", Hash1)` returns, as the provider
//! protocol's `volumes::stat` defines its `dirhash`. A directory
//! resource is its files: `dirhash` sees no empty directory, so none
//! is uploaded, and a directory with no file is not a resource.
//!
//! # How a resource reaches a container
//!
//! Over FUSE, from the daemon, and no other way: a template's —
//! an agent's or a tool's —
//! [`ResourceFileMount`](crate::daemon::template::ResourceFileMount)
//! or [`ResourceDirectoryMount`](crate::daemon::template::ResourceDirectoryMount)
//! names a resource at a container path, read-only or ephemeral, and
//! the daemon answers the mount's asks from the bytes it holds. A
//! resource is not a provider's volume and is mounted as none; nothing
//! writes a resource into a container's own filesystem before the
//! agent runs. What a caller wants a container to own it puts in a
//! volume.

mod kind;

pub use kind::*;

pub mod delete;
pub mod list;
pub mod upload;
