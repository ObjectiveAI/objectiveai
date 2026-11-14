//! Whether the caller holds an image, and under what name.
//!
//! Asked only on runs, by the provider of the caller. The provider
//! opens a channel with a [`request::Request`] carrying the run's own
//! [`Image`](crate::shared::containers::request::Image) — the same
//! argument the provider endpoint
//! [`images::read`](crate::provider::endpoints::images::read) takes —
//! and the caller answers with one [`response::Frame`] — the byte
//! `0`, not held, or the byte `1` and a name — then the finish; the
//! empty finish is not held. A provider that would take an image from
//! the caller asks this before it asks for anything of the image, so
//! a caller that does not hold it is not asked for a manifest it
//! cannot give. The name is a label: the provider puts it in the
//! reference of the one pull it serves the image for, and never
//! stores it or reads anything into it. Holding is proven by
//! [`prove`](super::prove), not by this answer.

pub mod request;
pub mod response;
