//! Answering the two messages scopes.
//!
//! [`listen`] keeps the caller's mailbox open and hands it every
//! message sent to it; [`send`] posts one message and answers its fate.
//! Both go through the provider's
//! [`Mailboxes`](crate::protocol::mailboxes::Mailboxes).

pub mod listen;
pub mod send;
