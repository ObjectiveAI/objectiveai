//! A set of tags, kept sorted.

/// `tags` with every one of `added` in it: a sorted, deduplicated
/// list, as the wire reports tags and the store keeps them. A tag
/// held already is held still.
pub fn with(tags: &[String], added: &[String]) -> Vec<String> {
    let mut all: Vec<String> = tags.iter().chain(added).cloned().collect();
    all.sort();
    all.dedup();
    all
}

/// `tags` with every one of `removed` taken out, sorted and
/// deduplicated. A tag not held is nothing to take.
pub fn without(tags: &[String], removed: &[String]) -> Vec<String> {
    let mut kept: Vec<String> = tags.iter().filter(|tag| !removed.contains(tag)).cloned().collect();
    kept.sort();
    kept.dedup();
    kept
}
