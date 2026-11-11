//! Toolset definitions — each capability and its arguments.
//!
//! AUTH IS A SECRET here exactly as it is for
//! [providers](super::provider): a toolset's credentials are never
//! arguments. Each structure carries the facts that are not secret —
//! an endpoint, a backend's name, a switch — and its `SECRET:`
//! paragraph names the secrets those facts select, which the agent's
//! template declares by name and the proxy feeds before every loop;
//! a static credential (an api key, a token) is `fixed`, rotating
//! state (spotify's login) is `rotating`. Toolsets with nothing to
//! configure are `Option<bool>`s on [`Toolsets`] (absent is off) and
//! get no file — including the ones whose only argument was a key,
//! since `true` now declares the secret: a structure with no fields
//! would only be ceremony. Nothing about a tool's auth rides the
//! filesystem or a mount, and the request carries no environment for
//! it to ride.
//!
//! Where two toolsets name the same secret (both generation tools
//! declare `FAL_KEY`; x_search and the xai provider both declare
//! `XAI_API_KEY`), a name declared twice is ONE secret — one value
//! fed, applied once.
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
//! rotation is what a rotating secret is for. Hermes's managed Nous
//! tool gateway (a rotating credential several toolsets can route
//! through) stays unsupported with the Nous provider.

pub mod browser;
pub mod homeassistant;
pub mod spotify;
pub mod tts;
pub mod web;

mod toolsets;

pub use toolsets::*;
