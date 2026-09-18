//! Asking a container what its arguments may be.
//!
//! The exchange that makes an image usable without knowing it: the
//! caller opens a channel that carries nothing — a direction with
//! nothing to say has no request here, as everywhere in this crate —
//! and the provider answers with one [`response::Frame`] — the JSON
//! Schema of the
//! [`arguments`](crate::shared::containers::request::Container::arguments)
//! the container was made with, or an error — then the finish. Every
//! container scope carries it, on the same tag, whichever kind of
//! container it is.
//!
//! The schema is the ARGUMENTS', not the request's. The rest of the
//! request is this crate's to type; the arguments are the image's to
//! define, and this is how the image says what it takes. A schema
//! constrains a document, which is exactly what that value is: one
//! value, sent once. What it cannot say — how a loop's chunks are
//! ordered, what a tool server's tools do — is not its job, and this
//! crate says that in prose as it does everywhere.

pub mod response;
