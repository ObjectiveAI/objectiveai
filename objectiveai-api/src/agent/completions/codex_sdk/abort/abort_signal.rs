/// Cancellation primitive. Mirrors `AbortSignal` in `abort.py:18-51`.
///
/// **Data definition only.** Python carries an `asyncio.Event` and exposes
/// `wait()` / `throw_if_aborted()` methods on top of these two fields; the
/// runtime cancellation behavior lives in the future task that wires up the
/// upstream-client layer (Task #1). This struct exists so other types
/// (`AbortController`, `TurnOptions`) can hold the same shape Python does.
///
/// `reason` is `Option<serde_json::Value>` because Python's `Optional[Any]`
/// allows arbitrary user-supplied payloads.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AbortSignal {
    pub aborted: bool,
    pub reason: Option<serde_json::Value>,
}
