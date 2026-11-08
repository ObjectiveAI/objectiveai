//! The requests over the daemons reached through the providers,
//! served: [`discover`] lists the identities discoverable through
//! them, [`listen`] hears the messages sent to the daemon through
//! them, and [`message`] sends one through them. Each is judged by the
//! providers grant, and reaches only the providers that grant reaches.

pub mod discover;
pub mod listen;
pub mod message;
