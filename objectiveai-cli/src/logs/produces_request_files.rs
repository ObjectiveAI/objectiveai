//! `ProducesRequestFiles` — the request-side analog of the response
//! chunks' `produce_files` methods.
//!
//! Each request `*CreateParams` type implements this trait to break
//! itself out into per-field log files (messages, response_format,
//! continuation, input, state, …) instead of being serialized as one
//! monolithic JSON blob. The [`crate::filesystem::logs::LogWriter`]'s
//! `with_request` consumes this trait to produce the on-disk request
//! log.
//!
//! The returned `LogReference` points at the top-level
//! `<route_base>/<id>.json` summary file that holds the
//! `*CreateParamsLog` (refs to its sub-files). The `Vec<LogFile>`
//! includes the summary file itself plus every per-field child.

use crate::filesystem::logs::{LogFile, LogReference};

pub trait ProducesRequestFiles {
    /// Walk the request, write each extracted sub-field to its own
    /// [`LogFile`] (collected into the returned vec), and return a
    /// [`LogReference`] pointing at the top-level summary file.
    ///
    /// `route_base` is the route prefix the writer hands down —
    /// typically `"<endpoint>/request"` after the suffix→subdir
    /// flip (Phase 2 of the request-log refactor).
    fn produce_files(
        &self,
        id: &str,
        route_base: &str,
    ) -> (LogReference, Vec<LogFile>);
}
