//! `LogReference` — re-exported from the SDK so the on-disk pointer
//! shape lives in one place. The SDK's `*Log` data types need to
//! express references in their fields; both the SDK and the CLI use
//! [`objectiveai_sdk::LogReference`] for that.

pub use objectiveai_sdk::{LogReference, LogReferenceTag};
