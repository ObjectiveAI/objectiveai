//! The requests over images, served: [`info`], an image's DIVERGE.md,
//! from a container of it running or from a provider; and [`oci`], the
//! image itself, from a provider or a remote daemon.

pub mod info;
pub mod oci;
