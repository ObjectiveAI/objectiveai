//! Toolset definitions — each capability and its arguments.
//!
//! AUTH IS AN ARGUMENT here exactly as it is for
//! [providers](super::provider): every toolset that takes anything
//! is a structure in its own file carrying that tool's credentials
//! and endpoint facts — STATIC values (api keys, tokens, urls) as
//! plain fields, ROTATING state as `*_resource` identity fields
//! the resource machinery serves. Toolsets with nothing to
//! configure are `Option<bool>`s on [`Toolsets`] (absent is off)
//! and get no file: a structure with no fields would only be
//! ceremony.
//! Nothing about a tool's auth
//! rides the filesystem or a mount, and the request carries no
//! environment for it to ride.
//!
//! Where two toolsets name the same underlying variable (both
//! generation tools apply `FAL_KEY`), a request supplying both MUST
//! agree with itself — disagreement is the caller's contradiction,
//! and the harness may refuse it.
//!
//! DROPPED from the previous vocabulary:
//! - `context_engine` — ships zero tools in stock Hermes; the
//!   toolset is a socket for an engine plugin the image does not
//!   carry, so there is nothing to enable.
//! - `yuanbao` — its tools call a live Yuanbao platform adapter
//!   that an api_server-only gateway never creates.
//! - `delegation` — sub-agents. On the API server every delegation
//!   is BACKGROUND: the run ends before its children do, and their
//!   results wake the session as a turn nobody drives, off every
//!   stream a run has. Observable only through end-of-turn hooks,
//!   which is a different product than a stream — so the harness
//!   turns the toolset OFF, and it is not a switch a request can
//!   throw.
//!
//! Still absent for their unchanged non-auth reasons: skills
//! (mounts decide it — directories under
//! `/root/.hermes/external-skills/`, each holding a `SKILL.md`, which
//! the harness names to Hermes read-only), `computer_use` (a display
//! server is not an
//! argument), `stt` (not a model toolset at all), `clarify` (needs
//! wiring this wire does not have), `discord`/`discord_admin`
//! (hard-restricted to Hermes's Discord platform), `cronjob` (an
//! ephemeral container has no future to schedule into). `spotify`
//! LEFT this list: its only blocker was rotating OAuth, and
//! rotation is what resources are for. Hermes's managed Nous tool
//! gateway (a rotating credential several toolsets can route
//! through) stays unsupported with the Nous provider.

pub mod browser;
pub mod homeassistant;
pub mod image_gen;
pub mod spotify;
pub mod tts;
pub mod video_gen;
pub mod web;
pub mod x_search;

mod toolsets;

pub use toolsets::*;
