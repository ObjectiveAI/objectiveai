//! The five daemon endpoints, served: a daemon accepting connections
//! through this provider, and a daemon connecting to one; the listing
//! of the listable identities; and the two messaging scopes.
//!
//! [`listen`] keeps the caller's mailbox open and hands it every
//! message sent to it; [`message`] posts one message and answers its
//! fate. Both go through the provider's
//! [`Mailboxes`](crate::protocol::mailboxes::Mailboxes).

pub mod accept;
pub mod connect;
pub mod list;
pub mod listen;
pub mod message;
