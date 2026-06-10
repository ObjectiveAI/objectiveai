//! Asserts that `json_schemas()` exports no duplicate schemas.
//!
//! The builder writes one file per schema, so two identical entries
//! would silently collide on disk. We detect duplicates by inserting
//! every exported schema into a set. `schemars::Schema` wraps a
//! `serde_json::Value`, which does not implement `Hash`, so the set is
//! keyed on each schema's canonical serialized form rather than on the
//! `Schema` value directly.

use std::collections::HashSet;

#[test]
fn json_schemas_have_no_duplicates() {
    let mut seen: HashSet<String> = HashSet::new();
    let mut duplicates: Vec<String> = Vec::new();

    for schema in objectiveai_sdk::json_schemas() {
        let serialized =
            serde_json::to_string(&schema).expect("schema serializes to JSON");
        if !seen.insert(serialized) {
            // Report the duplicate itself — its title when present,
            // otherwise the full serialized schema. There's no need to
            // identify which earlier entry it duplicates.
            let label = serde_json::to_value(&schema)
                .ok()
                .and_then(|v| {
                    v.get("title").and_then(|t| t.as_str()).map(str::to_string)
                })
                .unwrap_or_else(|| serde_json::to_string(&schema).unwrap());
            duplicates.push(label);
        }
    }

    assert!(
        duplicates.is_empty(),
        "duplicate schemas in json_schemas(): {duplicates:?}"
    );
}
