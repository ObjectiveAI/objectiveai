//! `logs/` — CLI-side ports of every filesystem-coupled helper that
//! used to live on SDK types under the deleted `#[cfg(feature =
//! "filesystem")]` gates. The internal tree mirrors the SDK source
//! paths exactly, with one rename rule applied at the top: SDK
//! singular leading segments become plural (`agent/` → `agents/`,
//! `vector/` → `vectors/`). Already-plural segments pass through.
//!
//! Each relocated function takes the SDK type by value or ref as its
//! first parameter (replacing `self`) and returns the same tuple shape
//! the SDK method did.
//!
//! Also hosts the [`ProducesRequestFiles`] trait (relocated from
//! `filesystem/logs/`), since every `*CreateParams::produce_files`
//! impl now lives under this mirror.

pub mod agents;
pub mod cli;
pub mod functions;
pub mod produces_request_files;
pub mod vectors;

pub use cli::Commands;
pub use produces_request_files::ProducesRequestFiles;
