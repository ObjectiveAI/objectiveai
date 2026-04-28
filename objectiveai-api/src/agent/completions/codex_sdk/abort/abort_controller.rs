/// Controller that owns an [`super::AbortSignal`] and can flip it. Mirrors
/// `AbortController` in `abort.py:54-63`.
///
/// **Data definition only.** Python's `abort(reason)` method is runtime
/// behavior; that lives in the future cancellation wiring (Task #1).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AbortController {
    pub signal: super::AbortSignal,
}
