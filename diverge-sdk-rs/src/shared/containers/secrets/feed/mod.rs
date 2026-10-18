//! Feeding a container its secrets' values: what a `secrets` channel
//! answers.
//!
//! A `secrets` channel is opened with [`Values`](super::Values) and
//! nothing in front of them — the tag that says which channel it is
//! belongs to the frame that carries it, the caller's on a run and
//! the provider's on a begin — and answered with one
//! [`response::Frame`]: fed, or why not; then the channel finishes.
//! Opened at every activation of a container whose program declared
//! at least one secret, after the values are locked or read and
//! before anything is delivered to the program; a refusal is the
//! delivery's error. The same frame answers the caller on the run and
//! the provider on the begin, so it is defined once here.

pub mod response;
