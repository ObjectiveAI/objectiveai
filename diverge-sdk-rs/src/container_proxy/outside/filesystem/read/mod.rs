//! Reading one file out of the container.
//!
//! Split by who SENDS: the path goes in [`client`], the bytes come
//! back in [`server`]. The server names one file; the proxy answers
//! its bytes in pieces and finishes, or an error last. One question,
//! one stream back, and no channel on either side.

pub mod client;
pub mod server;
