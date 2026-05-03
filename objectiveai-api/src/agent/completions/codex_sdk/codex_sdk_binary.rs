/// Embedded `objectiveai-codex-sdk-runner` binary, baked in by the
/// crate's `build.rs` when the `codex-sdk` feature is on. Extracted to a
/// per-version temp dir on first use by [`super::Client::binary_path`].
#[cfg(feature = "codex-sdk")]
pub const CODEX_SDK_RUNNER: &[u8] =
    include_bytes!(env!("OBJECTIVEAI_CODEX_SDK_RUNNER_PATH"));
