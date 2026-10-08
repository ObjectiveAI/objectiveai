//! Uploading files into a tool's container.
//!
//! A client opens the scope saying what is coming — one file at a path,
//! or a directory of named files at a path — and the daemon asks for
//! each file's content on a channel of its own; the client answers each
//! in pieces and finishes it, and when every channel has finished the
//! daemon answers once that the files are in place, that no tool is the
//! one named, forbidden, or that it failed, and the scope finishes.
//! Each file lands whole, every parent made, replacing what was at its
//! path and touching nothing else; a content channel that ends in an
//! error abandons that file, and the answer is the error. A created
//! tool's container that is not running is started for the operation
//! and stopped when it finishes; one the daemon has running anyway — an
//! attached agent active — is used as it runs. A connected tool is
//! joined for the operation through the provider protocol's
//! `containers::tools::connect` and left when it finishes. Nothing else
//! about the tool changes.
//!
//! The content travels on channels the daemon opens, since only a
//! responder can end a channel: the client could not say which piece
//! was its last.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
