//! The six daemon endpoints, served: a daemon accepting connections
//! through this provider, and a daemon connecting to one; a daemon
//! announcing itself, and the listing of the announced identities; and
//! the two messaging scopes.
//!
//! [`announce`] keeps the caller in the provider's
//! [`Listables`](crate::protocol::listables::Listables) for as long as
//! its scope is open, and [`list`] lists them. [`listen`] keeps the
//! caller's mailbox open and hands it every message sent to it;
//! [`message`] posts one message and answers its fate. Both go through
//! the provider's [`Mailboxes`](crate::protocol::mailboxes::Mailboxes),
//! and neither needs its caller announced.

pub mod accept;
pub mod announce;
pub mod connect;
pub mod list;
pub mod listen;
pub mod message;
