//! Client-side agent tags. Bodies are stubbed; SQL lands in stage 6.

use super::{Error, Pool};

/// Three-state result for a tag-name lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LookupState {
    Bound {
        agent_instance_hierarchy: String,
    },
    Pending {
        parent_agent_instance_hierarchy: String,
        agent_full_id: String,
    },
    Absent,
}

/// Parent scope of an `agent_instance_hierarchy`: the substring up to
/// (but not including) the last `/`. When the input has no `/`, the
/// parent is the empty string.
pub fn parent_of(agent_instance_hierarchy: &str) -> &str {
    match agent_instance_hierarchy.rfind('/') {
        Some(i) => &agent_instance_hierarchy[..i],
        None => "",
    }
}

/// Leaf segment of an `agent_instance_hierarchy`: everything after the
/// last `/`. When the input has no `/`, the leaf is the whole string.
pub fn leaf_of(agent_instance_hierarchy: &str) -> &str {
    match agent_instance_hierarchy.rfind('/') {
        Some(i) => &agent_instance_hierarchy[i + 1..],
        None => agent_instance_hierarchy,
    }
}

/// Upsert `name` into PENDING state. Re-tags by displacing any prior
/// row (BOUND or PENDING) at the same `name`.
pub async fn upsert_pending(
    _pool: &Pool,
    _name: &str,
    _agent_full_id: &str,
    _parent_agent_instance_hierarchy: &str,
) -> Result<(), Error> {
    unimplemented!("db::tags::upsert_pending — stage 6")
}

/// Upsert `name` into BOUND state. Re-tags by displacing any prior row
/// at the same `name`.
pub async fn upsert_bound(
    _pool: &Pool,
    _name: &str,
    _agent_instance_hierarchy: &str,
) -> Result<(), Error> {
    unimplemented!("db::tags::upsert_bound — stage 6")
}

/// All tags currently bound to the given hierarchy, newest-bound first.
pub async fn tags_for_hierarchy(
    _pool: &Pool,
    _agent_instance_hierarchy: &str,
) -> Result<Vec<String>, Error> {
    unimplemented!("db::tags::tags_for_hierarchy — stage 6")
}

/// Hierarchy bound to a given tag. `None` for PENDING or absent rows.
pub async fn hierarchy_for_tag(
    _pool: &Pool,
    _name: &str,
) -> Result<Option<String>, Error> {
    unimplemented!("db::tags::hierarchy_for_tag — stage 6")
}

/// Look up a tag and report its precise state.
pub async fn lookup(_pool: &Pool, _name: &str) -> Result<LookupState, Error> {
    unimplemented!("db::tags::lookup — stage 6")
}

/// First-chunk notification: flip every PENDING row whose
/// `(agent_full_id, parent_agent_instance_hierarchy)` matches
/// `(agent_full_id, parent_of(agent_instance_hierarchy))` to BOUND
/// against the full `agent_instance_hierarchy`. Returns the names of
/// every promoted tag.
pub async fn upgrade(
    _pool: &Pool,
    _agent_full_id: &str,
    _agent_instance_hierarchy: &str,
) -> Result<Vec<String>, Error> {
    unimplemented!("db::tags::upgrade — stage 6")
}
