use std::fs;
use std::path::Path;

fn load_schemas() -> Vec<(String, serde_json::Value)> {
    let schema_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut schemas = Vec::new();
    for entry in fs::read_dir(&schema_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let content: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
            let name = path.file_name().unwrap().to_str().unwrap().to_string();
            schemas.push((name, content));
        }
    }
    assert!(!schemas.is_empty(), "no schema files found");
    schemas
}

const ALLOWED_KEYWORDS: &[&str] = &[
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
    "omitempty",
];

fn collect_keywords(value: &serde_json::Value, inside_properties: bool, found: &mut std::collections::BTreeSet<String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                if !inside_properties {
                    found.insert(k.clone());
                }
                collect_keywords(v, k == "properties", found);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                collect_keywords(v, false, found);
            }
        }
        _ => {}
    }
}

fn check_keyword_order(value: &serde_json::Value, inside_properties: bool, errors: &mut Vec<String>, path: &str) {
    match value {
        serde_json::Value::Object(map) => {
            if !inside_properties {
                let keys: Vec<&String> = map.keys().collect();
                let mut max_pos: Option<usize> = None;
                for key in &keys {
                    if let Some(pos) = ALLOWED_KEYWORDS.iter().position(|k| *k == key.as_str()) {
                        if let Some(prev) = max_pos {
                            if pos < prev {
                                errors.push(format!("{path}: \"{key}\" is out of order"));
                            }
                        }
                        max_pos = Some(max_pos.map_or(pos, |p| p.max(pos)));
                    }
                }
            }
            for (k, v) in map {
                let child_path = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                check_keyword_order(v, k == "properties", errors, &child_path);
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                check_keyword_order(v, false, errors, &format!("{path}[{i}]"));
            }
        }
        _ => {}
    }
}

fn check_properties_sorted(
    value: &serde_json::Value,
    inside_properties: bool,
    errors: &mut Vec<String>,
    path: &str,
) {
    match value {
        serde_json::Value::Object(map) => {
            // Only treat `properties` as the JSON-Schema keyword (whose
            // keys are user field names that must be alphabetical) when
            // we're at a schema level. When `inside_properties` is true,
            // the current map's keys ARE user field names — so a key
            // literally named "properties" here is a user field whose
            // value is itself a sub-schema (not a property map).
            if !inside_properties {
                if let Some(serde_json::Value::Object(props)) = map.get("properties") {
                    let keys: Vec<&String> = props.keys().collect();
                    for w in keys.windows(2) {
                        if w[0] > w[1] {
                            errors.push(format!(
                                "{path}.properties: \"{0}\" comes before \"{1}\" but should come after",
                                w[0], w[1]
                            ));
                        }
                    }
                }
            }
            for (k, v) in map {
                let child_path = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                let child_inside = if inside_properties {
                    false
                } else {
                    k == "properties"
                };
                check_properties_sorted(v, child_inside, errors, &child_path);
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                check_properties_sorted(v, false, errors, &format!("{path}[{i}]"));
            }
        }
        _ => {}
    }
}

fn has_type_array(value: &serde_json::Value, inside_properties: bool) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            if !inside_properties {
                if let Some(serde_json::Value::Array(_)) = map.get("type") {
                    return true;
                }
            }
            map.iter()
                .any(|(k, v)| has_type_array(v, k == "properties"))
        }
        serde_json::Value::Array(arr) => arr.iter().any(|v| has_type_array(v, false)),
        _ => false,
    }
}

fn has_key_recursive(value: &serde_json::Value, target_key: &str, inside_properties: bool) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                if !inside_properties && k == target_key {
                    return true;
                }
                if has_key_recursive(v, target_key, k == "properties") {
                    return true;
                }
            }
            false
        }
        serde_json::Value::Array(arr) => arr
            .iter()
            .any(|v| has_key_recursive(v, target_key, false)),
        _ => false,
    }
}

#[test]
fn only_allowed_keywords() {
    let allowed: std::collections::BTreeSet<String> =
        ALLOWED_KEYWORDS.iter().map(|s| s.to_string()).collect();
    let mut all_found = std::collections::BTreeSet::new();
    for (_, schema) in load_schemas() {
        collect_keywords(&schema, false, &mut all_found);
    }
    let unexpected: Vec<_> = all_found.difference(&allowed).collect();
    assert!(
        unexpected.is_empty(),
        "unexpected keywords found: {unexpected:?}"
    );
}

#[test]
fn keywords_in_canonical_order() {
    for (name, schema) in load_schemas() {
        let mut errors = Vec::new();
        check_keyword_order(&schema, false, &mut errors, &name);
        assert!(errors.is_empty(), "keyword ordering violations:\n{}", errors.join("\n"));
    }
}

#[test]
fn properties_keys_sorted_alphabetically() {
    for (name, schema) in load_schemas() {
        let mut errors = Vec::new();
        check_properties_sorted(&schema, false, &mut errors, &name);
        assert!(errors.is_empty(), "properties sorting violations:\n{}", errors.join("\n"));
    }
}

#[test]
fn no_schema_property() {
    for (name, schema) in load_schemas() {
        assert!(
            !has_key_recursive(&schema, "$schema", false),
            "{name} contains a $schema property"
        );
    }
}

#[test]
fn no_type_arrays_outside_properties() {
    for (name, schema) in load_schemas() {
        assert!(
            !has_type_array(&schema, false),
            "{name} contains a type array outside of properties"
        );
    }
}

#[test]
fn no_required_outside_properties() {
    for (name, schema) in load_schemas() {
        assert!(
            !has_key_recursive(&schema, "required", false),
            "{name} contains a required key outside of properties"
        );
    }
}

#[test]
fn no_one_of_outside_properties() {
    for (name, schema) in load_schemas() {
        assert!(
            !has_key_recursive(&schema, "oneOf", false),
            "{name} contains a oneOf key outside of properties"
        );
    }
}

fn has_any_of_with_sibling(value: &serde_json::Value, sibling: &str, inside_properties: bool) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            if !inside_properties && map.contains_key("anyOf") && map.contains_key(sibling) {
                return true;
            }
            map.iter()
                .any(|(k, v)| has_any_of_with_sibling(v, sibling, k == "properties"))
        }
        serde_json::Value::Array(arr) => arr.iter().any(|v| has_any_of_with_sibling(v, sibling, false)),
        _ => false,
    }
}

#[test]
fn no_any_of_with_sibling_ref_outside_properties() {
    for (name, schema) in load_schemas() {
        assert!(
            !has_any_of_with_sibling(&schema, "$ref", false),
            "{name} has anyOf with a sibling $ref"
        );
    }
}

#[test]
fn no_any_of_with_sibling_type_outside_properties() {
    for (name, schema) in load_schemas() {
        assert!(
            !has_any_of_with_sibling(&schema, "type", false),
            "{name} has anyOf with a sibling type"
        );
    }
}

#[test]
fn no_any_of_with_sibling_properties_outside_properties() {
    for (name, schema) in load_schemas() {
        assert!(
            !has_any_of_with_sibling(&schema, "properties", false),
            "{name} has anyOf with a sibling properties"
        );
    }
}

#[test]
fn no_const_outside_properties() {
    for (name, schema) in load_schemas() {
        assert!(
            !has_key_recursive(&schema, "const", false),
            "{name} contains a const key outside of properties"
        );
    }
}

fn has_numeric_format(value: &serde_json::Value, inside_properties: bool) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            if !inside_properties {
                let is_numeric = matches!(
                    map.get("type").and_then(|t| t.as_str()),
                    Some("integer") | Some("number")
                );
                if is_numeric && map.contains_key("format") {
                    return true;
                }
            }
            map.iter()
                .any(|(k, v)| has_numeric_format(v, k == "properties"))
        }
        serde_json::Value::Array(arr) => arr.iter().any(|v| has_numeric_format(v, false)),
        _ => false,
    }
}

#[test]
fn no_numeric_format() {
    for (name, schema) in load_schemas() {
        assert!(
            !has_numeric_format(&schema, false),
            "{name} has a format key on an integer or number type"
        );
    }
}

fn check_format_values(
    value: &serde_json::Value,
    inside_properties: bool,
    errors: &mut Vec<String>,
    path: &str,
) {
    match value {
        serde_json::Value::Object(map) => {
            if !inside_properties {
                if let Some(serde_json::Value::String(fmt)) = map.get("format") {
                    if fmt != "uuid" && fmt != "date-time" {
                        errors.push(format!("{path}: format is \"{fmt}\" (expected \"uuid\" or \"date-time\")"));
                    }
                }
            }
            for (k, v) in map {
                let child_path = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                check_format_values(v, k == "properties", errors, &child_path);
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                check_format_values(v, false, errors, &format!("{path}[{i}]"));
            }
        }
        _ => {}
    }
}

#[test]
fn format_is_uuid_or_datetime_only() {
    for (name, schema) in load_schemas() {
        let mut errors = Vec::new();
        check_format_values(&schema, false, &mut errors, &name);
        assert!(
            errors.is_empty(),
            "format must be \"uuid\" or \"date-time\":\n{}",
            errors.join("\n")
        );
    }
}

fn collect_refs(value: &serde_json::Value, refs: &mut std::collections::BTreeSet<String>, inside_properties: bool) {
    match value {
        serde_json::Value::Object(map) => {
            if !inside_properties {
                if let Some(serde_json::Value::String(r)) = map.get("$ref") {
                    refs.insert(r.clone());
                }
            }
            for (k, v) in map {
                collect_refs(v, refs, k == "properties");
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                collect_refs(v, refs, false);
            }
        }
        _ => {}
    }
}

fn check_min_max(value: &serde_json::Value, inside_properties: bool, errors: &mut Vec<String>, path: &str) {
    match value {
        serde_json::Value::Object(map) => {
            if !inside_properties {
                if let (Some(min), Some(max)) = (map.get("minimum"), map.get("maximum")) {
                    if let (Some(min_f), Some(max_f)) = (min.as_f64(), max.as_f64()) {
                        if min_f > max_f {
                            errors.push(format!("{path}: minimum ({min_f}) > maximum ({max_f})"));
                        }
                    }
                }
            }
            for (k, v) in map {
                let child_path = if path.is_empty() { k.clone() } else { format!("{path}.{k}") };
                check_min_max(v, k == "properties", errors, &child_path);
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                check_min_max(v, false, errors, &format!("{path}[{i}]"));
            }
        }
        _ => {}
    }
}

#[test]
fn minimum_never_exceeds_maximum() {
    for (name, schema) in load_schemas() {
        let mut errors = Vec::new();
        check_min_max(&schema, false, &mut errors, &name);
        assert!(errors.is_empty(), "min/max violations:\n{}", errors.join("\n"));
    }
}

fn check_multi_variant_anyof_not_nullable(
    value: &serde_json::Value,
    inside_properties: bool,
    errors: &mut Vec<String>,
    path: &str,
) {
    match value {
        serde_json::Value::Object(map) => {
            if !inside_properties {
                if let Some(serde_json::Value::Array(variants)) = map.get("anyOf") {
                    let non_null_count = variants
                        .iter()
                        .filter(|v| v.get("type").and_then(|t| t.as_str()) != Some("null"))
                        .count();
                    let has_null = variants.iter().any(|v| {
                        v.get("type").and_then(|t| t.as_str()) == Some("null")
                    });
                    if non_null_count >= 2 && has_null {
                        errors.push(format!(
                            "{path}: anyOf has {non_null_count} non-null variants plus a null variant"
                        ));
                    }
                }
            }
            for (k, v) in map {
                let child_path = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                check_multi_variant_anyof_not_nullable(v, k == "properties", errors, &child_path);
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                check_multi_variant_anyof_not_nullable(v, false, errors, &format!("{path}[{i}]"));
            }
        }
        _ => {}
    }
}

#[test]
fn multi_variant_anyof_never_nullable() {
    for (name, schema) in load_schemas() {
        let mut errors = Vec::new();
        check_multi_variant_anyof_not_nullable(&schema, false, &mut errors, &name);
        assert!(
            errors.is_empty(),
            "anyOf with 2+ non-null variants must not include a null variant:\n{}",
            errors.join("\n")
        );
    }
}

fn check_no_nested_null_in_anyof(
    value: &serde_json::Value,
    inside_properties: bool,
    errors: &mut Vec<String>,
    path: &str,
) {
    match value {
        serde_json::Value::Object(map) => {
            if !inside_properties {
                if let Some(serde_json::Value::Array(variants)) = map.get("anyOf") {
                    for (i, variant) in variants.iter().enumerate() {
                        // Check if this variant has its own anyOf containing null
                        if let Some(serde_json::Value::Array(inner_variants)) =
                            variant.get("anyOf")
                        {
                            if inner_variants.iter().any(|v| {
                                v.get("type").and_then(|t| t.as_str()) == Some("null")
                            }) {
                                errors.push(format!(
                                    "{path}.anyOf[{i}]: nested anyOf contains a null variant (null must be at the outer level)"
                                ));
                            }
                        }
                    }
                }
            }
            for (k, v) in map {
                let child_path = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                check_no_nested_null_in_anyof(v, k == "properties", errors, &child_path);
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                check_no_nested_null_in_anyof(v, false, errors, &format!("{path}[{i}]"));
            }
        }
        _ => {}
    }
}

#[test]
fn no_nested_null_in_anyof_variants() {
    let mut errors = Vec::new();
    for (name, schema) in load_schemas() {
        check_no_nested_null_in_anyof(&schema, false, &mut errors, &name);
    }
    assert!(
        errors.is_empty(),
        "null variants must not be nested inside anyOf variants:\n{}",
        errors.join("\n")
    );
}

fn check_any_of_variants_have_title(
    value: &serde_json::Value,
    inside_properties: bool,
    errors: &mut Vec<String>,
    path: &str,
) {
    match value {
        serde_json::Value::Object(map) => {
            if !inside_properties {
                if let Some(serde_json::Value::Array(variants)) = map.get("anyOf") {
                    let non_null: Vec<(usize, &serde_json::Value)> = variants
                        .iter()
                        .enumerate()
                        .filter(|(_, v)| v.get("type").and_then(|t| t.as_str()) != Some("null"))
                        .collect();
                    // Only check multi-variant unions (not simple nullable anyOf)
                    if non_null.len() >= 2 {
                        for (i, variant) in &non_null {
                            if variant.get("title").and_then(|t| t.as_str()).is_none() {
                                errors.push(format!(
                                    "{path}.anyOf[{i}]: non-null variant is missing a title"
                                ));
                            }
                        }
                    }
                }
            }
            for (k, v) in map {
                let child_path = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                check_any_of_variants_have_title(v, k == "properties", errors, &child_path);
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                check_any_of_variants_have_title(v, false, errors, &format!("{path}[{i}]"));
            }
        }
        _ => {}
    }
}

#[test]
fn all_any_of_variants_have_title() {
    let mut errors = Vec::new();
    for (name, schema) in load_schemas() {
        check_any_of_variants_have_title(&schema, false, &mut errors, &name);
    }
    assert!(
        errors.is_empty(),
        "every non-null variant in a multi-variant anyOf must have a title:\n{}",
        errors.join("\n")
    );
}

fn is_valid_schema_title(title: &str) -> bool {
    let segments: Vec<&str> = title.split('.').collect();
    if segments.is_empty() {
        return false;
    }
    // Last segment must be PascalCase (starts with uppercase)
    let last = segments.last().unwrap();
    if !last.starts_with(|c: char| c.is_uppercase()) {
        return false;
    }
    // All preceding segments must be snake_case (starts with lowercase)
    segments[..segments.len() - 1]
        .iter()
        .all(|s| s.starts_with(|c: char| c.is_lowercase()))
}

#[test]
fn titles_are_snake_case_dot_pascal_case() {
    let mut bad = Vec::new();
    for (name, schema) in load_schemas() {
        let title = schema
            .get("title")
            .and_then(|t| t.as_str())
            .unwrap_or(&name);
        if !is_valid_schema_title(title) {
            bad.push(title.to_string());
        }
    }
    assert!(
        bad.is_empty(),
        "titles must be optional snake_case segments followed by a final PascalCase segment:\n{}",
        bad.join("\n")
    );
}

#[test]
fn all_refs_resolve() {
    let schemas = load_schemas();
    let all_titles: std::collections::BTreeSet<String> = schemas
        .iter()
        .filter_map(|(_, s)| s.get("title").and_then(|t| t.as_str()).map(String::from))
        .collect();
    let mut all_refs = std::collections::BTreeSet::new();
    for (_, schema) in &schemas {
        collect_refs(schema, &mut all_refs, false);
    }
    let unresolved: Vec<&String> = all_refs.difference(&all_titles).collect();
    assert!(
        unresolved.is_empty(),
        "$ref targets not found as schema titles: {unresolved:?}"
    );
}
