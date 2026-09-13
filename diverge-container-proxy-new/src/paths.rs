//! Paths the server names, as the filesystem's.

use std::path::PathBuf;

/// `/` joined with `components` — or `None` for a list that names no
/// file: an empty one, which is the root, or any component that is
/// not a name. A name is not empty, is not `.` or `..`, and holds no
/// `/` and no NUL; a component is a name, never an instruction, so
/// one of those is refused rather than obeyed.
pub fn absolute(components: &[String]) -> Option<PathBuf> {
    if components.is_empty() {
        return None;
    }
    let mut path = PathBuf::from("/");
    for component in components {
        if component.is_empty()
            || component == "."
            || component == ".."
            || component.contains(['/', '\0'])
        {
            return None;
        }
        path.push(component);
    }
    Some(path)
}
