//! Creating a container from an image.
//!
//! Split by who SENDS: the request and the answers to the provider's
//! channels are in [`client`], the provider's own traffic in
//! [`server`].
//!
//! The scope a creation opens is the container's LIFE. It carries the
//! image pull on channels the provider opens, then the container's
//! filesystem on channel `0` for as long as it runs — so a caller does
//! not create and then separately ask to watch, and does not hold a
//! handle to something it might have to remember to release.

pub mod client;
pub mod server;
