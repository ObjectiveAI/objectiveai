//! The requests over images, served: [`read`], the image itself and
//! everything read of it — its manifests, blobs, proof and DIVERGE.md
//! — from a provider or a remote daemon; and [`build`], one built on a
//! provider into the daemon's Store there.

pub mod build;
pub mod read;
