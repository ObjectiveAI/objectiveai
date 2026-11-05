//! The device codes issued and not yet spent, RFC 8628's.
//!
//! A device code is issued to the issuer's one app at the device
//! authorization endpoint, with a user code, for the subject the login
//! names as its `login_hint`. There is no sign-in: the issuer's operator,
//! on its host, decides. Its life, as rules:
//!
//! 1. It is `pending` from its issue until the operator approves it —
//!    `approved`, naming the subject it asked for, which comes to be an
//!    identity then if it was not one — or denies it — `denied`. Only a
//!    pending code that has not expired is decided — see
//!    [`approve`](Devices::approve) and [`deny`](Devices::deny); the
//!    ones waiting are [`waiting`](Devices::waiting).
//! 2. The app polls the token endpoint with the device code — see
//!    [`poll`](Devices::poll). A poll of a code that has expired
//!    removes it and is [`Poll::Expired`]. A poll of a pending code
//!    sooner than its interval after the last poll is
//!    [`Poll::SlowDown`], and adds five seconds to its interval, as
//!    RFC 8628 §3.5 requires; any other poll of a pending code is
//!    [`Poll::Pending`]. A poll of a denied code removes it and is
//!    [`Poll::Denied`]; of an approved one, removes it and is
//!    [`Poll::Approved`], once.
//! 3. A poll of a code there is no such one of is [`Poll::Unknown`].
//! 4. An expired code is removed at the next code's issue.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod add;
mod decide;
mod device;
mod devices;
mod poll;
mod waiting;

pub use device::*;
pub use devices::*;
pub use poll::*;
