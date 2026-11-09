//! The requests over the daemons reached through the providers,
//! served: [`list`] lists the identities listable through
//! them, [`listen`] hears the messages sent to the daemon through
//! them, and [`message`] sends one through them. Each is judged by the
//! providers grant, and reaches only the providers that grant reaches.

pub mod list;
pub mod listen;
pub mod message;
