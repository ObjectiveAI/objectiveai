//! The volumes the provider offers: the SDK's `VolumeManager` and
//! `Volume`, supplied by this crate.
//!
//! # What a volume is on disk
//!
//! A volume a client created is one file, `<store>/<identity>/<name>`
//! under the store it was created in: an ext4 filesystem in a sparse
//! image, formatted here in pure Rust and loop-mounted by podman when
//! a container mounts the volume. The file's length is the volume's
//! size — reserved, not taken, since the image is sparse — and its
//! birth time is when the volume came into being. Nothing is kept
//! beside it: what a listing reports is what the filesystem records
//! about the file, and what a stat reports is read out of the image.
//! An edit lengthens or shortens the file and resizes the filesystem
//! in it to match, with the system's `e2fsck` and `resize2fs`: the
//! Linux host's own, or the podman machine's on macOS and Windows.
//!
//! A fixed volume is a directory the configuration names, offered as
//! it is: its size is declared in the configuration, its creation
//! time is the directory's, and a container mounts the directory
//! itself.
//!
//! # What is here
//!
//! [`VolumeManager`] is the manager: the stores, the fixed volumes,
//! and every identity that has asked, each an [`Identity`] holding
//! its stored volumes by name, read from the stores the first time
//! the identity is named. [`Volume`] is one volume, the SDK's
//! `Volume`, carrying the SDK's lock as an atomic flag and, once
//! asked, what a walk found — [`Walked`], the bytes in use and the
//! `dirhash`. [`walk_directory`] is the walk of a fixed volume's
//! directory and [`walk_image`] the walk of a stored volume's image;
//! [`reserve_image`] and [`format_image`] are how an image is made,
//! at [`image_path`]; the filesystem in one is resized by
//! [`tools::resize`](crate::tools::resize). [`Reservation`] is the
//! stores and the bytes reserved in
//! each, kept in memory and taken by compare-and-swap, shared by the
//! manager that creates and every volume that grows or shrinks.
//! [`sparse`] marks a new image sparse where the filesystem needs
//! telling, [`ok`] says which names a volume may have, and [`Error`]
//! is what any of it fails with.
//!
//! Every method of both traits is implemented.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod identity;
mod image;
mod name;
mod reservation;
mod sparse;
mod volume;
mod volume_manager;
mod walk;

pub use error::*;
pub use identity::*;
pub use image::*;
pub use name::*;
pub use reservation::*;
pub use sparse::*;
pub use volume::*;
pub use volume_manager::*;
pub use walk::*;
