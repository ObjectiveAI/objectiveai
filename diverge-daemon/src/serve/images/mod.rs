//! The requests over images, served: [`read`], the image itself and
//! everything read of it — its manifests, blobs, proof and DIVERGE.md
//! — from a provider or a remote daemon; [`build`], one built on a
//! provider into the daemon's Store there; [`push`], one built already
//! put there, the provider's asks for it carried to the client; and
//! [`list`], what the daemon owns in its Stores, and [`get`], one image
//! of them.

pub mod build;
pub mod get;
pub mod list;
pub mod push;
pub mod read;
