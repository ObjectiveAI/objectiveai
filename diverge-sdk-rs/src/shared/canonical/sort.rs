//! Every object key sorted, at every depth.

/// Sort every object key in `value`, at every depth — inside nested
/// objects and inside arrays alike.
///
/// Needed because `serde_json` runs with `preserve_order` here, so a
/// `Map` keeps INSERTION order: `{"b":1,"a":2}` and `{"a":2,"b":1}`
/// are equal as data but serialize to different bytes, and an id is
/// a hash of those bytes. Sorting only the TOP level would leave
/// `{"opts":{"b":1,"a":2}}` uncanonical.
///
/// Array ELEMENT order is deliberately untouched: an array is ordered
/// data, and reordering it would change what the value says, not
/// just how it was spelled.
pub fn sort_object_keys(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (_, nested) in map.iter_mut() {
                sort_object_keys(nested);
            }
            map.sort_keys();
        }
        serde_json::Value::Array(items) => {
            for item in items {
                sort_object_keys(item);
            }
        }
        _ => {}
    }
}
