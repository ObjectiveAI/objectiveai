//! Fetching content the server is missing, by its identity.
//!
//! An agent names its skills and its agent definitions by dirhash —
//! a deterministic content identity — and the content itself lives
//! with the client, in folders of the client's own. A provider that
//! is asked to run what it does not hold opens a channel with a
//! [`request::Request`] naming the kind and the hash, and the client
//! answers with the directory itself: one [`response::Frame`] per
//! file, then the finish.
//!
//! # By hash, not by name
//!
//! The request carries no name. A name is the caller's label for a
//! thing and may point at different content tomorrow; the dirhash IS
//! the content, and a provider that fetched by name would have no way
//! to know it got what the request meant. The client indexes its
//! folders however it likes — what it must answer to is the hash.
//!
//! # Absence is the empty finish
//!
//! A client without the hash sends no frames and finishes the
//! channel, which is what this protocol already means by a deliberate
//! could-not-serve. There is no error vocabulary on this exchange:
//! nothing an error could say would change what the provider does
//! next, which is not run the agent.

pub mod request;
pub mod response;
