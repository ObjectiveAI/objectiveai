//! What can be done to a container once you have one.
//!
//! [`create`](crate::endpoints::laboratories::run) and
//! [`connect`](crate::endpoints::laboratories::connect) differ in how
//! they GET a container — one makes it and owns its life, the other
//! joins somebody else's and asks permission. Once attached, they do
//! the same things to it, and those things are here.
//!
//! [`read`] is one exchange. A write is TWO — [`write_path`] names the
//! destination, [`write_bytes`] carries the content — and they are
//! siblings here rather than halves of a `write` module because each
//! is a whole exchange with its own request and its own response,
//! travelling in opposite directions.
//!
//! [`transfer`] is a read and a write that never became either,
//! available only when both ends live on one provider.
//!
//! [`request`] is the odd one out: it is about getting a container
//! rather than using one. What it holds is the part of that ask which
//! does not vary between the kinds — which today is who supplies the
//! image, and nothing else.
//!
//! Unlike [`http`](super::http) and [`filetree`](super::filetree),
//! which are shapes any endpoint could ride, everything in this module
//! is about containers specifically. It is here rather than in one
//! endpoint because more than one of them needs it, not because it is
//! general.

pub mod read;
pub mod request;
pub mod transfer;
pub mod write_bytes;
pub mod write_path;
