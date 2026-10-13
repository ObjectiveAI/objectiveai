//! Uploading files into a dependency tool's container.
//!
//! A client opens the scope saying what is coming — one file at a path,
//! or a directory of named files at a path — and the daemon asks for
//! each file's content on a channel of its own; the client answers each
//! in pieces and finishes it, and when every channel has finished the
//! daemon answers once that the files are in place, that no dependency
//! tool is the one named or none runs, forbidden, or that it failed,
//! and the scope finishes. Each file lands whole, every parent made,
//! replacing what was at its path and touching nothing else; a content
//! channel that ends in an error abandons that file, and the answer is
//! the error. The tool's container is written as it runs: a dependency
//! tool that does not run is not started for the operation, and is
//! `NotFound`; nothing is stopped or released when the operation ends.
//! Nothing else about the tool changes.
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
