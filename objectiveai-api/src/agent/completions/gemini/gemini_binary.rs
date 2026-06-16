/// Embedded `objectiveai-gemini-sdk-runner` binary, baked in by the
/// crate's `build.rs`. Extracted to a per-version temp dir on first
/// use by [`super::Client::binary_path`].
pub const GEMINI_RUNNER: &[u8] =
    include_bytes!(env!("OBJECTIVEAI_GEMINI_SDK_RUNNER_PATH"));
