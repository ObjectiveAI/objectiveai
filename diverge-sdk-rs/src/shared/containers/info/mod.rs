//! The image's own word on what its arguments may be: `/DIVERGE.md`,
//! read by the proxy and answered to whoever asks.
//!
//! Every container image ships one file at the root of its
//! filesystem, `/DIVERGE.md`: a frontmatter between two `---` lines
//! that IS the JSON Schema of the
//! [`arguments`](crate::shared::containers::request::Container::arguments)
//! the image's program accepts, as raw JSON, and then a short markdown
//! body — what the image runs, and the secrets it declares.
//! [`Document`] is that file parsed, and [`Document::parse`] states
//! the rules; [`Document::render`] writes one back, so that one shape
//! is written and read.
//!
//! The exchange that makes an image usable without knowing it: the
//! caller opens a channel that carries nothing — a direction with
//! nothing to say has no request here, as everywhere in this crate —
//! and the provider answers with one [`response::Frame`]: the
//! document, parsed; that the image ships none; or an error. Then the
//! finish. Every container scope carries it, on the same tag,
//! whichever kind of container it is. The proxy inside the container
//! reads the file from the root at every ask and answers from what it
//! read; the program is never asked — the file is the image's, not the
//! program's — and an image that ships none is answered as missing,
//! which is a fact about the image and not a failure of anything.
//!
//! The schema is the ARGUMENTS', not the request's. The rest of the
//! request is this crate's to type; the arguments are the image's to
//! define, and the document is how the image says what it takes. A
//! schema constrains a document, which is exactly what that value is:
//! one value, sent once. What it cannot say — how a loop's chunks are
//! ordered, what a tool server's tools do — is not its job, and this
//! crate says that in prose as it does everywhere.

mod document;
mod parse_error;

pub mod response;

pub use document::*;
pub use parse_error::*;
