//! One handler per endpoint of the provider protocol, in the SDK's
//! own arrangement of them: the request each takes is the SDK's frame, the
//! answers it sends are the SDK's, and what it needs of a provider it
//! takes as the traits of [`protocol`](super). The dispatch in
//! [`handle`](super::handle) is what calls them.

pub mod containers;
pub mod daemons;
pub mod images;
pub mod version;
pub mod volumes;
