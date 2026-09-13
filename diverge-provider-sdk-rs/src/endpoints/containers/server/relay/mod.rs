//! The proxy's asks, carried to the caller and answered back.
//!
//! Every channel the proxy opens on the begin scope, and every ask a
//! mount makes on its own scope, arrives as the proxy's channel
//! number and an ask. Here each becomes a channel this end opens on
//! the run scope — the family's frame for it — whose answers become
//! channel responses on the proxy's channel, finished when the
//! caller finishes. One task per ask, so an answer that takes a while
//! holds up nothing else; the begin's asks ending is the proxy gone.
//! The agent's chunks ride the other way on no channel at all: off
//! the begin's main stream, onto the run's.

mod chunks;
mod fuse;
mod one;
mod postgres;
mod relay;
mod stream;

pub(crate) use chunks::*;
pub(crate) use fuse::*;
pub(crate) use relay::*;
