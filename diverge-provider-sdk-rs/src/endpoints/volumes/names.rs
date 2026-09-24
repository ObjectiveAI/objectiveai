//! Whether a path is one of names.

/// Whether every component of `path` is a name: not empty, not `.`
/// or `..`, and holding no `/` and no NUL. The rule every path in
/// this crate answers to, a mount's included: a provider descends
/// names and never resolves a path, so a component that is not a
/// name is not an instruction it could follow, and is refused before
/// anything is looked at. An empty path passes — it is the root — and
/// whether the root is allowed is the endpoint's to say.
pub fn ok(path: &[String]) -> bool {
    path.iter()
        .all(|name| !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\0']))
}
