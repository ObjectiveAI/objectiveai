//! Toolset definitions — each capability and its arguments.
//!
//! AUTH IS AN ARGUMENT here exactly as it is for
//! [providers](super::provider): each toolset is a structure in its
//! own file carrying that tool's credentials and endpoint facts —
//! STATIC values (api keys, tokens, urls) as plain fields, ROTATING
//! state as `*_resource` identity fields the resource machinery
//! serves. Nothing about a tool's auth rides the filesystem, a
//! mount, or the request's `environment`.
//!
//! Where two toolsets name the same underlying variable (both
//! generation tools apply `FAL_KEY`), a request supplying both MUST
//! agree with itself — disagreement is the caller's contradiction,
//! and the harness may refuse it.
//!
//! DROPPED from the previous vocabulary:
//! - `memory` — Hermes's own cross-session file store. Our
//!   cross-run memory is the CONTINUATION, and a second store that
//!   evaporates with the container would fight it. (External
//!   memory-provider plugins ride the same switch and leave with
//!   it.)
//! - `context_engine` — ships zero tools in stock Hermes; the
//!   toolset is a socket for an engine plugin the image does not
//!   carry, so there is nothing to enable.
//! - `yuanbao` — its tools call a live Yuanbao platform adapter
//!   that an api_server-only gateway never creates.
//!
//! Still absent for their unchanged non-auth reasons: skills
//! (mounts decide it), `computer_use` (a display server is not an
//! argument), `stt` (not a model toolset at all), `clarify` (needs
//! wiring this wire does not have), `discord`/`discord_admin`
//! (hard-restricted to Hermes's Discord platform), `cronjob` (an
//! ephemeral container has no future to schedule into). `spotify`
//! LEFT this list: its only blocker was rotating OAuth, and
//! rotation is what resources are for. Hermes's managed Nous tool
//! gateway (a rotating credential several toolsets can route
//! through) stays unsupported with the Nous provider.

pub mod browser;
pub mod code_execution;
pub mod delegation;
pub mod file;
pub mod homeassistant;
pub mod image_gen;
pub mod session_search;
pub mod spotify;
pub mod terminal;
pub mod todo;
pub mod tts;
pub mod video;
pub mod video_gen;
pub mod vision;
pub mod web;
pub mod x_search;

mod toolsets;

pub use toolsets::*;
