//! A run's life, as an agent's log and a tool's log both keep it.
//!
//! An agent's container and a tool's container are runs alike: each
//! is deployed on a provider, works and rests, and is taken down. Both
//! logs keep that life in the same four items, with the same JSON —
//! `deployed`, `active`, `inactive` and `undeployed` — and only the
//! daemon writes them. The rules:
//!
//! 1. **Deployed first.** [`Deployed`] is kept when the run's
//!    container is up on a provider, and before anything the run
//!    does. No `active` is ever kept before its run's `deployed`.
//! 2. **Active and inactive bracket work.** `active` is kept when the
//!    run begins working — an agent's loop runs, or an MCP exchange
//!    is in flight for it; a tool has an ask in flight — and
//!    `inactive` when it ceases. They alternate, `active` first,
//!    each kept only on a change.
//! 3. **Undeployed last.** [`Undeployed`] is kept when the run's
//!    container is down, however it went down, and names the provider
//!    its `deployed` named. A run that was working is first closed:
//!    each ask still open is answered as interrupted, then `inactive`
//!    is kept, then `undeployed` with `interrupted` true. No `active`
//!    and no `inactive` of a run is ever kept after its `undeployed`,
//!    and no ask of a run is kept or answered after it.
//! 4. **Expected or not.** A run taken down because it was asked to
//!    be — gone idle, its record deleted, its template's service
//!    retired — carries no `reason`. A run that ended unexpectedly
//!    carries exactly one [`Reason`], and that `reason` is the only
//!    place a log says so: a plain `error` never records a drop.
//! 5. **Brackets nest.** A log reads `deployed`, zero or more
//!    `active` … `inactive` stretches, `undeployed`, and again for the
//!    next run. A run that never came up keeps an `error` and no
//!    `deployed`.
//!
//! Its own files are flattened into it, so everything is named through
//! this module and not through the file it lives in.

mod deployed;
mod provider;
mod reason;
mod undeployed;

pub use deployed::*;
pub use provider::*;
pub use reason::*;
pub use undeployed::*;
