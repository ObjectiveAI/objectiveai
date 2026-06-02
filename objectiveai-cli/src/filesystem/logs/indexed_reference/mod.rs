//! Re-export of the SDK's `IndexedLogReference`, aliased as
//! `LogReference` so existing CLI callsites keep their
//! `indexed_reference::LogReference` import path.

pub use objectiveai_sdk::IndexedLogReference as LogReference;
