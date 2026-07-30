use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use syn::{Item, Visibility};
use walkdir::WalkDir;

/// Build a module prefix from a path like "src/functions/check/example_inputs/file.rs".
/// Strips "src/" prefix, drops the filename, then joins folder segments with dots
/// in lowercase. Returns e.g. "functions.check.example_inputs." (with trailing dot)
/// or "" for top-level files.
fn module_prefix(path: &str) -> String {
    let inner = path.strip_prefix("src/").unwrap_or(path);
    let segments: Vec<&str> = inner.split('/').collect();

    // Take only folder segments (skip the last segment which is the filename)
    let folders = &segments[..segments.len().saturating_sub(1)];
    if folders.is_empty() {
        String::new()
    } else {
        format!("{}.", folders.join("."))
    }
}

/// Modules whose JsonSchema-deriving types are intentionally skipped by
/// every coverage test in this file. Neither the MCP protocol types
/// (`src/mcp/`) nor the API↔CLI envelope shape used by the
/// `objectiveai-mcp` plugin runner (`src/client_objectiveai_mcp/`)
/// are REQUIRED in the published schema set: their
/// `JsonRpcResult<R>::Ok` arm schemas `R = ()` as bare
/// `{"type":"null"}`, which downstream Go / TS / Python SDK
/// generators can't reconstruct, and the broader MCP wire types are
/// documented externally by the MCP spec. A minimum subset IS shipped
/// (allowed — only `expected − actual` is checked): the request
/// params + result types (and their `$ref` closure) that the
/// `agents mcp` command leaves put on the wire, so the per-language
/// codegen can type those five leaves' execute functions. Everything
/// else still derives JsonSchema (locally documenting) without
/// shipping in `json_schemas()` and isn't checked for global
/// coverage.
fn is_skipped_module(relative: &str) -> bool {
    relative.starts_with("src/mcp/")
        || relative.starts_with("src/client_objectiveai_mcp/")
        || relative.starts_with("src/laboratories/daemon/")
        // Subprocess plumbing: the stdout readiness handshake
        // (`ServerReady`) and the daemon↔child stdin control channel
        // are internal spawn contracts between the daemon and its
        // leashed children — never published API shapes.
        || relative == "src/process.rs"
        || relative == "src/child_stdio.rs"
}

fn has_derive(attrs: &[syn::Attribute], trait_name: &str) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("derive") {
            let tokens = attr
                .meta
                .require_list()
                .ok()
                .map(|list| list.tokens.to_string());
            tokens.map_or(false, |t| {
                t.split(',').any(|s| {
                    s.split("::")
                        .last()
                        .map_or(false, |last| last.trim() == trait_name)
                })
            })
        } else {
            false
        }
    })
}

fn has_json_schema_derive(attrs: &[syn::Attribute]) -> bool {
    has_derive(attrs, "JsonSchema")
}

fn has_serde_derive(attrs: &[syn::Attribute]) -> bool {
    has_derive(attrs, "Serialize") || has_derive(attrs, "Deserialize")
}

/// Check whether a type has a manual `impl Serialize for TypeName` or
/// `impl ... Deserialize<'_> for TypeName` anywhere in the file.
fn has_manual_serde_impl(file: &syn::File, type_name: &str) -> bool {
    for item in &file.items {
        if let Item::Impl(impl_item) = item {
            // Check if the Self type matches
            let self_matches = match impl_item.self_ty.as_ref() {
                syn::Type::Path(tp) => tp
                    .path
                    .segments
                    .last()
                    .map_or(false, |seg| seg.ident == type_name),
                _ => false,
            };
            if !self_matches {
                continue;
            }
            // Check if this is a trait impl for Serialize or Deserialize
            if let Some((_, trait_path, _)) = &impl_item.trait_ {
                if let Some(last) = trait_path.segments.last() {
                    let name = last.ident.to_string();
                    if name == "Serialize" || name == "Deserialize" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// True when the item carries `#[json_schema_ignore]` (any path
/// spelling, e.g. `#[objectiveai_sdk_macros::json_schema_ignore]`) —
/// the explicit opt-out from ALL json-schema coverage rules. Used for
/// serializable wire types that deliberately ship without a
/// registered schema (e.g. the root `ResponseItem` aggregates, whose
/// schema expansion downstream TypeScript cannot serialize — TS7056).
fn has_json_schema_ignore(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path()
            .segments
            .last()
            .map_or(false, |seg| seg.ident == "json_schema_ignore")
    })
}

/// Returns the schema_override value if present: "Owned", "Ref", or "RefOwnedEnum".
fn get_schema_override(attrs: &[syn::Attribute]) -> Option<String> {
    attrs.iter().find_map(|attr| {
        if attr.path().is_ident("schema_override") {
            let list = attr.meta.require_list().ok()?;
            Some(list.tokens.to_string().trim().to_string())
        } else {
            None
        }
    })
}

fn get_schemars_rename(attrs: &[syn::Attribute]) -> Option<String> {
    attrs.iter().find_map(|attr| {
        if attr.path().is_ident("schemars") {
            let list = attr.meta.require_list().ok()?;
            let tokens = list.tokens.to_string();
            // The schemars attribute may contain additional meta items after
            // `rename = "..."` (e.g. `extend(...)`, `bound = "..."`), so we
            // can't rely on the closing `"` being the final character.
            let rest = tokens.strip_prefix("rename")?;
            let rest = rest.trim().strip_prefix('=')?;
            let rest = rest.trim().strip_prefix('"')?;
            let end = rest.find('"')?;
            Some(rest[..end].to_string())
        } else {
            None
        }
    })
}

/// Returns true if the item has type parameters (not lifetime or const params).
fn has_type_params(item: &Item) -> bool {
    let generics = match item {
        Item::Struct(s) => &s.generics,
        Item::Enum(e) => &e.generics,
        _ => return false,
    };
    generics
        .params
        .iter()
        .any(|p| matches!(p, syn::GenericParam::Type(_)))
}

/// Every public type that implements Serialize or Deserialize (via derive or
/// manual impl) must derive JsonSchema with the correct schemars rename.
/// Conversely, types that do NOT implement Serialize/Deserialize must NOT
/// derive JsonSchema.
#[test]
fn all_serializable_types_have_json_schema() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let source_root = Path::new(manifest_dir).join("src");

    let mut errors: Vec<String> = Vec::new();

    for entry in WalkDir::new(&source_root) {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }

        let relative = path
            .strip_prefix(manifest_dir)
            .unwrap()
            .to_str()
            .unwrap()
            .replace('\\', "/");

        if is_skipped_module(&relative) {
            continue;
        }

        let source = fs::read_to_string(path).unwrap();
        let file = match syn::parse_file(&source) {
            Ok(f) => f,
            Err(_) => continue,
        };

        let prefix = module_prefix(&relative);

        for item in &file.items {
            let (name, attrs) = match item {
                Item::Struct(s) if matches!(s.vis, Visibility::Public(_)) => {
                    (s.ident.to_string(), &s.attrs)
                }
                Item::Enum(e) if matches!(e.vis, Visibility::Public(_)) => {
                    (e.ident.to_string(), &e.attrs)
                }
                _ => continue,
            };

            if has_json_schema_ignore(attrs) {
                continue;
            }

            let full_name = format!("{prefix}{name}");
            let schema_override = get_schema_override(attrs);
            let is_serializable =
                has_serde_derive(attrs) || has_manual_serde_impl(&file, &name);
            let has_schema = has_json_schema_derive(attrs);

            // schema_override(Ref) and schema_override(RefOwnedEnum) must NOT derive JsonSchema
            if matches!(
                schema_override.as_deref(),
                Some("Ref") | Some("RefOwnedEnum")
            ) {
                if has_schema {
                    errors.push(format!(
                        "{name} in {relative} has #[schema_override({0})] \
                         but must not derive JsonSchema",
                        schema_override.as_ref().unwrap()
                    ));
                }
                continue;
            }

            // schema_override(Owned): must derive JsonSchema, rename must strip "Owned" suffix
            if schema_override.as_deref() == Some("Owned") {
                if !has_schema {
                    errors.push(format!(
                        "{name} in {relative} has #[schema_override(Owned)] \
                         but is missing #[derive(JsonSchema)]"
                    ));
                    continue;
                }
                let expected_rename =
                    full_name.strip_suffix("Owned").unwrap_or(&full_name);
                match get_schemars_rename(attrs) {
                    Some(rename) if rename == expected_rename => {}
                    Some(rename) => {
                        errors.push(format!(
                            "{name} in {relative} has #[schema_override(Owned)] \
                             with wrong rename: got \"{rename}\", expected \"{expected_rename}\""
                        ));
                    }
                    None => {
                        errors.push(format!(
                            "{name} in {relative} has #[schema_override(Owned)] \
                             but is missing #[schemars(rename = \"{expected_rename}\")]"
                        ));
                    }
                }
                continue;
            }

            if is_serializable && !has_schema {
                errors.push(format!(
                    "{name} in {relative} implements Serialize/Deserialize \
                     but is missing #[derive(JsonSchema)]"
                ));
                continue;
            }

            if !is_serializable && has_schema {
                errors.push(format!(
                    "{name} in {relative} has #[derive(JsonSchema)] but does \
                     not implement Serialize or Deserialize"
                ));
                continue;
            }

            // From here on, only check rename for types that have JsonSchema
            if !has_schema {
                continue;
            }

            // For types with type parameters, the rename contains {T} placeholders,
            // so we check the prefix matches rather than exact equality.
            let has_type_param = has_type_params(item);

            match get_schemars_rename(attrs) {
                None => {
                    errors.push(format!(
                        "{name} in {relative} is missing \
                         #[schemars(rename = \"{full_name}\")]"
                    ));
                }
                Some(rename) if has_type_param => {
                    let expected_prefix = format!("{full_name}.{{");
                    if !rename.starts_with(&expected_prefix) {
                        errors.push(format!(
                            "{name} in {relative} has wrong schemars rename: \
                             got \"{rename}\", expected \"{full_name}.{{...}}\""
                        ));
                    }
                }
                Some(rename) if rename != full_name => {
                    errors.push(format!(
                        "{name} in {relative} has wrong schemars rename: \
                         got \"{rename}\", expected \"{full_name}\""
                    ));
                }
                _ => {}
            }
        }
    }

    if !errors.is_empty() {
        panic!(
            "JsonSchema coverage errors ({}):\n{}",
            errors.len(),
            errors.join("\n")
        );
    }
}

/// Verifies that `json_schemas()` returns a schema for every non-generic
/// public struct/enum that derives JsonSchema.
#[test]
fn json_schemas_covers_all_types() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let source_root = Path::new(manifest_dir).join("src");

    // Collect all expected full_names from AST walking, skipping generic types
    let mut expected: BTreeSet<String> = BTreeSet::new();

    for entry in WalkDir::new(&source_root) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let relative = path
            .strip_prefix(manifest_dir)
            .unwrap()
            .to_str()
            .unwrap()
            .replace('\\', "/");
        if is_skipped_module(&relative) {
            continue;
        }
        let source = fs::read_to_string(path).unwrap();
        let file = match syn::parse_file(&source) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let prefix = module_prefix(&relative);
        for item in &file.items {
            let (name, attrs) = match item {
                Item::Struct(s) if matches!(s.vis, Visibility::Public(_)) => {
                    (s.ident.to_string(), &s.attrs)
                }
                Item::Enum(e) if matches!(e.vis, Visibility::Public(_)) => {
                    (e.ident.to_string(), &e.attrs)
                }
                _ => continue,
            };
            if !has_json_schema_derive(attrs) {
                continue;
            }
            if has_json_schema_ignore(attrs) {
                continue;
            }
            // Skip types with type parameters — their titles contain
            // concrete substitutions that don't match the template name.
            if has_type_params(item) {
                continue;
            }
            // For schema_override(Owned), use the schemars rename as the expected title
            let full_name =
                if get_schema_override(attrs).as_deref() == Some("Owned") {
                    match get_schemars_rename(attrs) {
                        Some(rename) => rename,
                        None => format!("{prefix}{name}"),
                    }
                } else {
                    format!("{prefix}{name}")
                };
            expected.insert(full_name);
        }
    }

    // Collect titles from json_schemas()
    let schemas = objectiveai_sdk::json_schemas();
    let mut actual: BTreeSet<String> = BTreeSet::new();
    for schema in &schemas {
        let json = serde_json::to_value(schema).unwrap();
        if let Some(title) = json.get("title").and_then(|t| t.as_str()) {
            actual.insert(title.to_string());
        }
    }

    let missing: Vec<&String> = expected.difference(&actual).collect();
    if !missing.is_empty() {
        panic!(
            "Types in source but missing from json_schemas() ({}):\n{}",
            missing.len(),
            missing
                .iter()
                .map(|m| format!("  - {m}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// Collects all `$ref` targets from a JSON value recursively.
fn collect_refs(value: &serde_json::Value, refs: &mut BTreeSet<String>) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::String(r)) = map.get("$ref") {
                if let Some(name) = r.strip_prefix("#/$defs/") {
                    refs.insert(name.to_string());
                }
            }
            for v in map.values() {
                collect_refs(v, refs);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                collect_refs(v, refs);
            }
        }
        _ => {}
    }
}

/// Verifies that every `$ref` used across all schemas returned by `json_schemas()`
/// exists as a title of some schema in the returned set.
#[test]
fn json_schemas_refs_are_complete() {
    let schemas = objectiveai_sdk::json_schemas();

    // Collect all titles
    let mut all_titles: BTreeSet<String> = BTreeSet::new();
    for schema in &schemas {
        let json = serde_json::to_value(schema).unwrap();
        if let Some(title) = json.get("title").and_then(|t| t.as_str()) {
            all_titles.insert(title.to_string());
        }
    }

    // Collect all $ref targets across all schemas
    let mut all_refs: BTreeSet<String> = BTreeSet::new();
    for schema in &schemas {
        let json = serde_json::to_value(schema).unwrap();
        collect_refs(&json, &mut all_refs);
    }

    // Every $ref target must exist as a title of some schema
    let unresolved: Vec<&String> = all_refs.difference(&all_titles).collect();

    if !unresolved.is_empty() {
        panic!(
            "$ref targets not found as titles in json_schemas() ({}):\n{}",
            unresolved.len(),
            unresolved
                .iter()
                .map(|r| format!("  - {r}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}
