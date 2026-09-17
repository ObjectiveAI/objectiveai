//! Which names a volume may have.

/// Whether `name` may be a volume's: one path component that names a
/// file in a store and nothing else. Not empty, not beginning with
/// `.`, and holding no `/`, `\` or NUL. `.` and `..` are refused by
/// the second rule, and so is any dotfile a store might hold that is
/// not a volume: a store's entries are volumes exactly when their
/// names pass this.
pub fn ok(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.') && !name.contains(['/', '\\', '\0'])
}
