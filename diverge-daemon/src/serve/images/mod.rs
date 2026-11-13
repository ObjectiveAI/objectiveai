//! The requests over images, served: [`info`], an image's DIVERGE.md,
//! from a container of it running or from a provider; [`oci`], the
//! image itself, from a provider or a remote daemon; and [`build`],
//! one built on a provider into the daemon's Store there.

pub mod build;
pub mod info;
pub mod oci;
