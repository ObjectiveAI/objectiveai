//! The daemons reached through the providers: who is listable on
//! them, and the direct messages sent through them.
//!
//! [`list`] lists the identities listable through the
//! connected providers: what a [`daemons`](crate::daemon::endpoints::daemons)
//! record names a remote by. [`listen`] hears the messages sent to the
//! daemon through them, and [`message`] sends one through them.
//!
//! The rules:
//!
//! 1. **Judged over providers.** Each of the three is judged by the
//!    `providers_daemons` grant — its `list`, `listen` and `message` actions —
//!    and not by a grant over identities: what each reaches is the
//!    providers the grant reaches.
//! 2. **Only those providers.** A listing holds only the routes through
//!    the providers the grant reaches, a listener hears only the
//!    messages that came through them, and a message is sent through no
//!    other.

pub mod list;
pub mod listen;
pub mod message;
