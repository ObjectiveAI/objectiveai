//! Driving the gateway's run.
//!
//! The container talks to `hermes gateway` through its API server:
//! `POST /v1/runs` starts a run, `GET /v1/runs/{run_id}/events` is
//! its event stream. [`raw`] is that transport and nothing more —
//! the run posted, the stream subscribed, each frame yielded as the
//! [`Event`](crate::response::Event) it parses to, unchanged. What
//! those events become on the container's own wire is the layer
//! above this one, still to come.

pub mod raw;
