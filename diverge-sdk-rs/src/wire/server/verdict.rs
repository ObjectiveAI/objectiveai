//! What a scope's answer came to, as far as whether it was allowed.

/// The first word on whether a scope was served, told once to whoever
/// set a [`Hook`] on its [`ScopeHandle`](super::scope_handle::ScopeHandle).
///
/// The rules:
///
/// 1. [`Allowed`](Self::Allowed): the server said so itself, with
///    [`allowed`](super::scope_handle::ScopeHandle::allowed), the moment
///    whatever judges the request let it through — for a scope whose
///    first answer is long in coming.
/// 2. [`Response`](Self::Response): the first response frame's payload,
///    as it goes out, for the hook to read.
/// 3. [`Nothing`](Self::Nothing): the scope finished, or was let go,
///    with no response before it.
///
/// Whichever comes first is told, and nothing after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict<'a> {
    /// The server let the request through.
    Allowed,
    /// The first response's payload.
    Response(&'a [u8]),
    /// No response came before the scope ended.
    Nothing,
}

/// What is told the [`Verdict`]: called once, and never under a lock.
pub type Hook = Box<dyn for<'a> FnOnce(Verdict<'a>) + Send>;
