//! The image's own word on what its arguments may be: `/DIVERGE.md`,
//! read of the image and answered on the image's own scope.
//!
//! Every container image ships one file at the root of its
//! filesystem, `/DIVERGE.md`: a frontmatter between two `---` lines
//! that is one JSON object — `name`, what the image
//! calls itself, and `schema`, the JSON Schema of the
//! [`arguments`](crate::shared::containers::request::Container::arguments)
//! the image's program accepts, both required; `frontend` where the
//! program serves one; and, in an agent's image, `chat` and
//! `structured_decisions`, each `false` or what the agent says of
//! itself ([`Chat`], [`StructuredDecisions`]) — and then a short markdown body: what
//! the image runs, and the secrets it declares.
//! [`Document`] is that file parsed, and [`Document::parse`] states
//! the rules; [`Document::render`] writes one back, so that one shape
//! is written and read.
//!
//! The exchange that makes an image usable without knowing it: on an
//! [`images::read`](crate::provider::endpoints::images::read) scope
//! that holds the image, the caller opens a channel that carries
//! nothing — a direction with nothing to say has no request here, as
//! everywhere in this crate — and the provider answers with one
//! [`response::Frame`]: the document, parsed; that the image ships
//! none; or an error. Then the finish. It is asked of the image and of
//! nothing else: no container scope carries it, and no container is
//! run for it — the provider reads the file out of the image. The
//! program is never asked — the file is the image's, not the
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

mod chat;
mod document;
mod false_or;
mod modalities;
mod modality;
mod parse_error;
mod structured_decisions;

pub mod response;

pub use chat::*;
pub use document::*;
pub use false_or::*;
pub use modalities::*;
pub use modality::*;
pub use parse_error::*;
pub use structured_decisions::*;
