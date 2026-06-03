use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Returns true if the name represents a generic type instantiation.
/// A name is generic if any dot-separated segment that is NOT the last one
/// starts with an uppercase letter (PascalCase).
fn is_generic_name(name: &str) -> bool {
    let segments: Vec<&str> = name.split('.').collect();
    segments
        .iter()
        .take(segments.len().saturating_sub(1))
        .any(|s| s.starts_with(|c: char| c.is_uppercase()))
}

/// Replace `$ref: "#/$defs/<generic>"` with the inlined definition throughout `value`.
/// Returns true if any replacement was made.
fn inline_generic_refs(
    value: &mut serde_json::Value,
    generic_defs: &HashMap<String, serde_json::Value>,
) -> bool {
    let mut changed = false;
    match value {
        serde_json::Value::Object(map) => {
            // Check if this object is a bare `$ref` to a generic def
            let should_inline = map
                .get("$ref")
                .and_then(|v| v.as_str())
                .and_then(|r| r.strip_prefix("#/$defs/"))
                .and_then(|name| generic_defs.get(name).cloned());

            if let Some(mut def) = should_inline {
                // Preserve description sibling to $ref, merging with def's description
                let sibling_desc = map.get("description").and_then(|v| v.as_str()).map(String::from);
                if let Some(serde_json::Value::Object(inlined)) = Some(&mut def).filter(|v| v.is_object()) {
                    let def_desc = inlined.get("description").and_then(|v| v.as_str()).map(String::from);
                    let merged = match (sibling_desc, def_desc) {
                        (Some(s), Some(d)) => Some(format!("{s}\n\n{d}")),
                        (Some(s), None) => Some(s),
                        (None, Some(d)) => Some(d),
                        (None, None) => None,
                    };
                    if let Some(desc) = merged {
                        inlined.insert("description".to_string(), serde_json::Value::String(desc));
                    }
                }
                *value = def;
                // The inlined value might itself have refs, recurse
                inline_generic_refs(value, generic_defs);
                return true;
            }

            // Recurse into all values
            for (_, v) in map.iter_mut() {
                if inline_generic_refs(v, generic_defs) {
                    changed = true;
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr.iter_mut() {
                if inline_generic_refs(v, generic_defs) {
                    changed = true;
                }
            }
        }
        _ => {}
    }
    changed
}

/// Inline generic `$defs` into the schema, replacing `$ref` with the actual definition.
/// Non-generic defs are kept in `$defs` for normalize to handle.
fn inline_generic_defs(schema: &mut serde_json::Value) {
    let map = match schema.as_object_mut() {
        Some(m) => m,
        None => return,
    };
    let defs = match map.remove("$defs") {
        Some(serde_json::Value::Object(defs)) => defs,
        other => {
            if let Some(v) = other {
                map.insert("$defs".to_string(), v);
            }
            return;
        }
    };

    // Separate generic and non-generic defs
    let mut generic_defs: HashMap<String, serde_json::Value> = HashMap::new();
    let mut non_generic_defs = serde_json::Map::new();
    for (name, def) in defs {
        if is_generic_name(&name) {
            generic_defs.insert(name, def);
        } else {
            non_generic_defs.insert(name, def);
        }
    }

    // Recursively resolve generic defs that reference other generic defs.
    // Remove each key while resolving so self-references are skipped,
    // then iterate until no more changes occur (fixed point).
    loop {
        let mut changed = false;
        let keys: Vec<String> = generic_defs.keys().cloned().collect();
        for key in keys {
            let mut def = generic_defs.remove(&key).unwrap();
            if inline_generic_refs(&mut def, &generic_defs) {
                changed = true;
            }
            generic_defs.insert(key, def);
        }
        if !changed {
            break;
        }
    }

    // Inline generic refs throughout the main schema
    inline_generic_refs(schema, &generic_defs);

    // Put back non-generic defs (normalize will strip $defs and rewrite $ref)
    if !non_generic_defs.is_empty() {
        schema
            .as_object_mut()
            .unwrap()
            .insert("$defs".to_string(), serde_json::Value::Object(non_generic_defs));
    }
}

fn normalize(value: &mut serde_json::Value, inside_properties: bool, title: &str) {
    match value {
        serde_json::Value::Object(map) => {
            // JSON Schema spec lets a property's value be a bare boolean
            // (`true` = "any value allowed", `false` = "no value allowed"),
            // and schemars 1.0+ emits `true` for `serde_json::Value` fields
            // with no other constraints (e.g. `JsonRpcRequest::id`).
            // Normalize to the equivalent object shape (`{}` / `{"not": {}}`)
            // here so SDK generators downstream only ever have to walk one
            // shape. Done at this object's `properties` child rather than
            // mid-recursion because Rust fields literally named "properties"
            // produce a same-named map one level deeper, which a depth flag
            // would misclassify.
            if !inside_properties {
                if let Some(serde_json::Value::Object(props)) = map.get_mut("properties") {
                    for (_k, v) in props.iter_mut() {
                        if let serde_json::Value::Bool(b) = v {
                            *v = if *b {
                                serde_json::Value::Object(serde_json::Map::new())
                            } else {
                                serde_json::json!({"not": {}})
                            };
                        }
                    }
                }
            }
            if !inside_properties {
                map.remove("$defs");
                map.remove("$schema");
                // Convert oneOf → anyOf
                if let Some(one_of) = map.remove("oneOf") {
                    map.insert("anyOf".to_string(), one_of);
                }
                // Flatten single-variant anyOf: merge the variant's keys into the parent
                if let Some(serde_json::Value::Array(variants)) = map.remove("anyOf") {
                    if variants.len() == 1 {
                        if let Some(serde_json::Value::Object(inner)) = variants.into_iter().next()
                        {
                            for (k, v) in inner {
                                map.insert(k, v);
                            }
                        }
                    } else {
                        // Multi-variant anyOf with sibling `properties` /
                        // `type` at the parent: schemars produces this
                        // shape for a `#[serde(flatten)]` of an
                        // internally-tagged enum inside a struct (e.g.
                        // `Request { id, #[flatten] payload: Payload }`).
                        // Downstream codegens (`no_any_of_with_sibling_*`
                        // tests, all SDK generators) require the
                        // discriminator-bearing variants to be the
                        // schema's only top-level shape. Push the parent
                        // `properties` into each variant and drop the
                        // sibling `properties` / `type` at the parent.
                        let sibling_props = map.remove("properties");
                        let sibling_type = map.remove("type");
                        let merged: Vec<serde_json::Value> = variants
                            .into_iter()
                            .map(|mut variant| {
                                if let serde_json::Value::Object(vmap) =
                                    &mut variant
                                {
                                    if let Some(serde_json::Value::Object(p)) =
                                        &sibling_props
                                    {
                                        let vprops = vmap
                                            .entry("properties".to_string())
                                            .or_insert_with(|| {
                                                serde_json::Value::Object(
                                                    serde_json::Map::new(),
                                                )
                                            });
                                        if let serde_json::Value::Object(vp) =
                                            vprops
                                        {
                                            for (k, v) in p {
                                                vp.entry(k.clone())
                                                    .or_insert_with(|| v.clone());
                                            }
                                        }
                                    }
                                    if let Some(t) = &sibling_type {
                                        vmap.entry("type".to_string())
                                            .or_insert_with(|| t.clone());
                                    }
                                }
                                variant
                            })
                            .collect();
                        map.insert(
                            "anyOf".to_string(),
                            serde_json::Value::Array(merged),
                        );
                    }
                }
                // Post-flatten fixups (inlined keys may include $ref, required, const)
                if let Some(serde_json::Value::String(r)) = map.get_mut("$ref") {
                    if *r == "#" {
                        *r = title.to_string();
                    } else if let Some(name) = r.strip_prefix("#/$defs/") {
                        *r = name.to_string();
                    }
                }
                map.remove("required");
                // Convert const → single-element enum
                if let Some(const_val) = map.remove("const") {
                    map.insert(
                        "enum".to_string(),
                        serde_json::Value::Array(vec![const_val]),
                    );
                }
                // Convert type: [T, "null"] → anyOf: [{type: T, ...constraints}, {type: "null"}]
                if let Some(serde_json::Value::Array(types)) = map.get("type") {
                    let types: Vec<String> = types
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect();
                    let non_null: Vec<&str> =
                        types.iter().map(|s| s.as_str()).filter(|t| *t != "null").collect();
                    let has_null = types.iter().any(|t| t == "null");
                    if has_null && non_null.len() == 1 {
                        map.remove("type");
                        // Partition siblings: type-specific constraints go on the inner
                        // schema, metadata (description, default) stays on the outer.
                        let mut inner = serde_json::Map::new();
                        inner.insert(
                            "type".to_string(),
                            serde_json::Value::String(non_null[0].to_string()),
                        );
                        let constraint_keys: &[&str] = &[
                            "items",
                            "properties",
                            "additionalProperties",
                            "minimum",
                            "maximum",
                            "format",
                            "pattern",
                            "minItems",
                            "maxItems",
                            "enum",
                        ];
                        for &key in constraint_keys {
                            if let Some(v) = map.remove(key) {
                                inner.insert(key.to_string(), v);
                            }
                        }
                        let null_variant = serde_json::json!({"type": "null"});
                        map.insert(
                            "anyOf".to_string(),
                            serde_json::Value::Array(vec![
                                serde_json::Value::Object(inner),
                                null_variant,
                            ]),
                        );
                    }
                }
                // Resolve numeric format → minimum/maximum, then delete format
                match map.get("type").and_then(|t| t.as_str()) {
                    Some("integer") => {
                        let format = map.remove("format")
                            .and_then(|v| v.as_str().map(String::from));
                        let (default_min, default_max): (i128, i128) = match format.as_deref() {
                            Some("int8") => (i8::MIN as i128, i8::MAX as i128),
                            Some("int16") => (i16::MIN as i128, i16::MAX as i128),
                            Some("int32") | Some("int") => (i32::MIN as i128, i32::MAX as i128),
                            Some("int64") | None => (i64::MIN as i128, i64::MAX as i128),
                            Some("int128") => (i128::MIN, i128::MAX),
                            Some("uint8") => (u8::MIN as i128, u8::MAX as i128),
                            Some("uint16") => (u16::MIN as i128, u16::MAX as i128),
                            Some("uint32") | Some("uint") => (u32::MIN as i128, u32::MAX as i128),
                            Some("uint64") => (u64::MIN as i128, u64::MAX as i128),
                            Some("uint128") => (u128::MIN as i128, i128::MAX), // u128::MAX exceeds i128
                            Some(_) => (i64::MIN as i128, i64::MAX as i128),
                        };
                        if !map.contains_key("minimum") {
                            map.insert("minimum".to_string(), serde_json::json!(default_min));
                        }
                        if !map.contains_key("maximum") {
                            map.insert("maximum".to_string(), serde_json::json!(default_max));
                        }
                    }
                    Some("number") => {
                        map.remove("format");
                        if !map.contains_key("minimum") {
                            map.insert("minimum".to_string(), serde_json::json!(f32::MIN));
                        }
                        if !map.contains_key("maximum") {
                            map.insert("maximum".to_string(), serde_json::json!(f32::MAX));
                        }
                    }
                    _ => {}
                }
            }
            for (k, v) in map.iter_mut() {
                // Keys inside "properties" are field names, not JSON
                // Schema keywords. Once we descend into a properties
                // map, the *children* of that map are field schemas
                // (not properties maps) — so the (k == "properties")
                // check below would mis-classify a Rust field literally
                // named "properties" (e.g. ToolSchemaObject.properties)
                // as a properties map. Force false at that level.
                let child_in_properties = if inside_properties { false } else { k == "properties" };
                normalize(v, child_in_properties, title);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                normalize(v, false, title);
            }
        }
        _ => {}
    }
}

const KEYWORD_ORDER: &[&str] = &[
    "title",
    "description",
    "type",
    "enum",
    "anyOf",
    "$ref",
    "properties",
    "additionalProperties",
    "items",
    "minItems",
    "maxItems",
    "minimum",
    "maximum",
    "pattern",
    "format",
    "default",
];

fn order_keys(value: &mut serde_json::Value, inside_properties: bool) {
    match value {
        serde_json::Value::Object(map) => {
            // Recurse first. Same caveat as `normalize`: a Rust field
            // literally named "properties" should not be treated as a
            // properties map when sorting its own children.
            for (k, v) in map.iter_mut() {
                let child_in_properties = if inside_properties { false } else { k == "properties" };
                order_keys(v, child_in_properties);
            }
            // Reorder this object's keys
            let entries: Vec<(String, serde_json::Value)> = map.into_iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            map.clear();
            if inside_properties {
                // Property field names: sort alphabetically
                let mut sorted = entries;
                sorted.sort_by(|(a, _), (b, _)| a.cmp(b));
                for (k, v) in sorted {
                    map.insert(k, v);
                }
            } else {
                // Schema keywords: sort by canonical order
                let mut sorted = entries;
                sorted.sort_by_key(|(k, _)| {
                    KEYWORD_ORDER.iter().position(|kw| kw == k).unwrap_or(usize::MAX)
                });
                for (k, v) in sorted {
                    map.insert(k, v);
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                order_keys(v, false);
            }
        }
        _ => {}
    }
}

fn main() {
    let out_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");

    // Clear only .json files (preserve builder/, CLAUDE.md, etc.)
    for entry in fs::read_dir(&out_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            fs::remove_file(&path).unwrap();
        }
    }

    let schemas = objectiveai_sdk::json_schemas();
    let mut count = 0;

    for schema in &schemas {
        let mut json = serde_json::to_value(schema).unwrap();
        let title = json
            .get("title")
            .and_then(|t| t.as_str())
            .unwrap_or_else(|| panic!("schema missing title: {json}"))
            .to_string();

        // Generic types are inlined into referencing schemas, not written as files.
        if is_generic_name(&title) {
            continue;
        }

        inline_generic_defs(&mut json);
        normalize(&mut json, false, &title);
        order_keys(&mut json, false);

        let filename = format!("{title}.json");
        let path = out_dir.join(&filename);
        let contents = serde_json::to_string_pretty(&json).unwrap();
        fs::write(&path, format!("{contents}\n")).unwrap();
        count += 1;
    }

    println!("Wrote {count} schema files to {}", out_dir.display());
}
