//! What the server sends back on a begin, one module per kind of
//! channel the proxy opens.
//!
//! Every one answers the CONTAINER, relayed: [`postgres`] is what its
//! database said, [`daemon`] the server frames that answer a frame of
//! its daemon connection — each an alias of the shape
//! [`shared`](crate::shared) defines.

pub mod daemon;
pub mod postgres;
