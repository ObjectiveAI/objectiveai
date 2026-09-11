//! Wire types for the Diverge broker API.
//!
//! This crate is the **normative artifact** of the broker
//! specification, the same way
//! [`diverge-provider-sdk`](https://docs.rs/diverge-provider-sdk) is of
//! the provider's. The types defined here are not a description of the
//! protocol written alongside an implementation — they are the
//! protocol. Prose documents the requirements a broker must satisfy;
//! it does not define the messages. Where the two disagree, this crate
//! is correct and the prose is a bug.
//!
//! # What a broker is for
//!
//! Answering who somebody is. A [`provider`] runs work for callers it
//! has no way to recognize on its own — a laboratory's runner, a
//! plugin's caller, an agent's owner are all just peers on a socket —
//! and a broker is what turns a peer into somebody.
//!
//! Eventually it will also answer who pays whom. Nothing about payment
//! is defined here, and nothing here should be read as anticipating a
//! shape for it: a field added now against a design that does not exist
//! is a field the design has to be bent around later.
//!
//! # Scope
//!
//! Types only — no transport, no client, no server. A broker
//! implementation binds these types to a transport; keeping this crate
//! free of runtime concerns is what lets it be depended on by every
//! side of the protocol, and by tools that only ever inspect it.
//!
//! # Its relationship to the provider protocol
//!
//! Separate crates because they are separate protocols with separate
//! peers. A provider never talks to a broker on a caller's behalf, and
//! a caller's dealings with a broker are settled before a provider
//! hears from it.
//!
//! What they may come to share is a vocabulary rather than a
//! dependency. The provider protocol already carries an `Identity`
//! describing on whose behalf a plugin runs, assembled by whoever
//! built the request — if a broker is what issues those facts, the two
//! will have to agree on them, and that agreement is worth writing
//! down once when it exists rather than guessed at now.
//!
//! # Status
//!
//! Bootstrapped and empty. The types land as the broker API is
//! defined.
//!
//! [`provider`]: https://docs.rs/diverge-provider-sdk
