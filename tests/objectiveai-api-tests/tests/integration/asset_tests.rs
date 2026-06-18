use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Asserts that every JSON file in `assets/` (excluding `assets/mock/`)
/// lives in a directory whose name ends with `client_tests` — or in
/// any nested subdirectory of one (e.g. `*client_tests/inputs/`).
#[test]
fn asset_json_files_live_in_client_tests_dir() {
    let assets_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let mock_dir = assets_dir.join("mock");

    let mut violations = Vec::new();

    for path in json_files(&assets_dir, &mock_dir) {
        // Walk up the file's ancestors and accept the path if any
        // ancestor's directory name ends with `client_tests`.
        let mut ok = false;
        for ancestor in path.ancestors() {
            if !ancestor.starts_with(&assets_dir) {
                break;
            }
            if let Some(name) = ancestor.file_name().and_then(|n| n.to_str()) {
                if name.ends_with("client_tests") {
                    ok = true;
                    break;
                }
            }
        }

        if !ok {
            violations.push(path.strip_prefix(&assets_dir).unwrap().to_path_buf());
        }
    }

    assert!(
        violations.is_empty(),
        "JSON files not under a `*client_tests/` directory:\n{}",
        format_paths(&violations),
    );
}

/// Asserts that every JSON file in `assets/` (excluding `assets/mock/`)
/// is referenced by an `include_str!` somewhere in `src/` *or* `tests/`.
///
/// Extracts all `include_str!` literal paths from source files,
/// resolves them to absolute paths, and checks every asset file is in
/// that set. For macro-generated paths using
/// `concat!("prefix", $base, "_N.json")`, expands by finding `$base`
/// values at call sites.
#[test]
fn asset_json_files_included_in_src() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let assets_dir = manifest_dir.join("assets");
    let mock_dir = assets_dir.join("mock");
    let src_dir = manifest_dir.join("src");
    let tests_dir = manifest_dir.join("tests");

    let mut included = collect_include_str_paths(&src_dir, manifest_dir);
    included.extend(collect_include_str_paths(&tests_dir, manifest_dir));

    let mut missing = Vec::new();

    for path in json_files(&assets_dir, &mock_dir) {
        let canonical = dunce::canonicalize(&path).unwrap_or_else(|_| path.clone());
        if !included.contains(&canonical) {
            missing.push(path.strip_prefix(&assets_dir).unwrap().to_path_buf());
        }
    }

    assert!(
        missing.is_empty(),
        "Asset JSON files not referenced by include_str! in src/ or tests/:\n{}",
        format_paths(&missing),
    );
}

/// Iterates all `.json` files under `assets_dir`, excluding `mock_dir`.
fn json_files(assets_dir: &Path, mock_dir: &Path) -> Vec<PathBuf> {
    walkdir::WalkDir::new(assets_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
        .filter(|e| !e.path().starts_with(mock_dir))
        .map(|e| e.into_path())
        .collect()
}

/// Collects canonical paths of all files referenced by `include_str!` in `.rs`
/// source files under `src_dir`.
///
/// Handles:
/// - `include_str!("relative/path.json")` — resolved relative to the source file.
/// - `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/path.json"))` — resolved
///   relative to the manifest directory.
/// - `include_str!(concat!("prefix/", $base, "_N.json"))` /
///   `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/prefix/", $base, "_N.json"))`
///   inside macro definitions — expanded by finding `$base` string
///   literal values at each macro call site (anywhere in the scanned
///   tree) and generating `_0` through `_9` suffixes.
fn collect_include_str_paths(src_dir: &Path, manifest_dir: &Path) -> HashSet<PathBuf> {
    let mut paths = HashSet::new();

    // Pattern: include_str!("literal_path")
    let direct_re = regex::Regex::new(r#"include_str!\(\s*"([^"]+)"\s*\)"#).unwrap();

    // Pattern: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/path"))
    let manifest_concat_re = regex::Regex::new(
        r#"include_str!\(\s*concat!\(\s*env!\("CARGO_MANIFEST_DIR"\)\s*,\s*"([^"]+)"\s*\)\s*\)"#,
    )
    .unwrap();

    // Pattern inside macro_rules!:
    //   include_str!(concat!("prefix", $base, "_N.json"))
    // We extract (prefix, suffix_template, base_relative_to: file_dir).
    let macro_concat_re = regex::Regex::new(
        r#"include_str!\(\s*concat!\(\s*"([^"]+)"\s*,\s*\$base\s*,\s*"([^"]+)"\s*\)\s*\)"#,
    )
    .unwrap();

    // Pattern inside macro_rules!:
    //   include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/prefix", $base, "_N.json"))
    // We extract (prefix, suffix_template, base_relative_to: manifest_dir).
    let macro_manifest_concat_re = regex::Regex::new(
        r#"include_str!\(\s*concat!\(\s*env!\("CARGO_MANIFEST_DIR"\)\s*,\s*"([^"]+)"\s*,\s*\$base\s*,\s*"([^"]+)"\s*\)\s*\)"#,
    )
    .unwrap();

    // Pattern: macro_rules! name { ... }
    let macro_def_re = regex::Regex::new(r"macro_rules!\s+(\w+)").unwrap();

    /// (prefix, suffix_template, base_dir): when expanding `prefix +
    /// base + suffix_template`, resolve the resulting relative path
    /// relative to `base_dir`.
    enum Anchor {
        FileDir,
        Manifest,
    }
    struct Template {
        macro_name: String,
        prefix: String,
        suffix_template: String,
        anchor: Anchor,
    }

    // First pass: scan every .rs file, collecting direct include_strs
    // (resolved immediately) and macro template definitions (deferred).
    let mut templates: Vec<(PathBuf, Template)> = Vec::new();
    let mut all_files: Vec<(PathBuf, String)> = Vec::new();

    for entry in walkdir::WalkDir::new(src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|x| x.to_str()) != Some("rs") {
            continue;
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let file_dir = path.parent().unwrap().to_path_buf();
        let path_buf = path.to_path_buf();

        // Direct include_str!("...")
        for cap in direct_re.captures_iter(&content) {
            let resolved = file_dir.join(&cap[1]);
            if let Ok(c) = dunce::canonicalize(&resolved) {
                paths.insert(c);
            }
        }

        // concat!(env!("CARGO_MANIFEST_DIR"), "/...")
        for cap in manifest_concat_re.captures_iter(&content) {
            let resolved = manifest_dir.join(cap[1].trim_start_matches('/'));
            if let Ok(c) = dunce::canonicalize(&resolved) {
                paths.insert(c);
            }
        }

        // Macro template patterns. Tag each template with the macro
        // name it sits inside so we can match it to invocations later.
        let macro_def_starts: Vec<(String, usize)> = macro_def_re
            .captures_iter(&content)
            .map(|c| (c[1].to_string(), c.get(0).unwrap().start()))
            .collect();
        let nearest_macro_name = |idx: usize| -> Option<String> {
            macro_def_starts
                .iter()
                .filter(|(_, start)| *start < idx)
                .max_by_key(|(_, start)| *start)
                .map(|(name, _)| name.clone())
        };

        for cap in macro_concat_re.captures_iter(&content) {
            let m_idx = cap.get(0).unwrap().start();
            if let Some(macro_name) = nearest_macro_name(m_idx) {
                templates.push((
                    path_buf.clone(),
                    Template {
                        macro_name,
                        prefix: cap[1].to_string(),
                        suffix_template: cap[2].to_string(),
                        anchor: Anchor::FileDir,
                    },
                ));
            }
        }
        for cap in macro_manifest_concat_re.captures_iter(&content) {
            let m_idx = cap.get(0).unwrap().start();
            if let Some(macro_name) = nearest_macro_name(m_idx) {
                templates.push((
                    path_buf.clone(),
                    Template {
                        macro_name,
                        prefix: cap[1].to_string(),
                        suffix_template: cap[2].to_string(),
                        anchor: Anchor::Manifest,
                    },
                ));
            }
        }

        all_files.push((path_buf, content));
    }

    if templates.is_empty() {
        return paths;
    }

    // Second pass: for each (template, file) cross product, find the
    // template's macro invocations in the file and expand.
    let lit_re = regex::Regex::new(r#""([^"]+)""#).unwrap();
    for (_def_path, tmpl) in &templates {
        let call_re = regex::Regex::new(&format!(
            r#"(?s){}!\((.+?)\);"#,
            regex::escape(&tmpl.macro_name),
        ))
        .unwrap();

        for (call_path, call_content) in &all_files {
            let call_dir = call_path.parent().unwrap();
            for call_cap in call_re.captures_iter(call_content) {
                let args = &call_cap[1];
                let literals: Vec<String> = lit_re
                    .captures_iter(args)
                    .map(|c| c[1].to_string())
                    .collect();

                // The $base is typically the last string literal in
                // the invocation argument list.
                let Some(base) = literals.last() else { continue };

                let ext_start = tmpl
                    .suffix_template
                    .find('.')
                    .unwrap_or(tmpl.suffix_template.len());
                let ext = &tmpl.suffix_template[ext_start..]; // ".json"
                for i in 0..10 {
                    let full = format!(
                        "{}{}_{}{}",
                        tmpl.prefix, base, i, ext,
                    );
                    let resolved = match tmpl.anchor {
                        Anchor::FileDir => call_dir.join(&full),
                        Anchor::Manifest => {
                            manifest_dir.join(full.trim_start_matches('/'))
                        }
                    };
                    if let Ok(c) = dunce::canonicalize(&resolved) {
                        paths.insert(c);
                    }
                }
            }
        }
    }

    paths
}

/// Asserts that within each mock category (agents, swarms, functions, profiles),
/// no two JSON files are duplicates. Comparison strips the root-level `description`
/// field, then deep-sorts the value into a canonical form (objects sorted by key,
/// arrays sorted by serialized representation, depth-first).
#[test]
fn no_duplicate_mock_fixtures() {
    let mock_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/mock");

    let categories = ["agents", "functions", "profiles", "swarms"];
    let mut all_violations = Vec::new();

    for category in &categories {
        let dir = mock_dir.join(category);
        if !dir.exists() {
            continue;
        }

        let mut seen: Vec<(serde_json::Value, String)> = Vec::new();
        for entry in std::fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            let content = std::fs::read_to_string(&path).unwrap();
            let mut value: serde_json::Value = serde_json::from_str(&content)
                .unwrap_or_else(|e| panic!("invalid JSON in {}: {e}", path.display()));

            // Strip root-level description only.
            if let Some(obj) = value.as_object_mut() {
                obj.remove("description");
            }

            let canonical = deep_sort(value);
            let filename = path.file_name().unwrap().to_string_lossy().to_string();
            seen.push((canonical, filename));
        }

        // Find duplicates.
        for i in 0..seen.len() {
            for j in (i + 1)..seen.len() {
                if seen[i].0 == seen[j].0 {
                    all_violations.push(format!(
                        "{}/{} == {}/{}",
                        category, seen[i].1, category, seen[j].1,
                    ));
                }
            }
        }
    }

    assert!(
        all_violations.is_empty(),
        "duplicate mock fixtures (ignoring root description):\n  {}",
        all_violations.join("\n  "),
    );
}

/// Recursively sorts a JSON value into a canonical form (depth-first).
/// Objects: sorted by key (converted from Map to a sorted Vec of pairs stored
/// as a JSON array of `[key, value]` pairs for deterministic equality).
/// Arrays: each element is deep-sorted first, then the array is sorted by
/// the serialized string representation of each element.
fn deep_sort(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            // Sort entries by key, deep-sort each value.
            let mut entries: Vec<(String, serde_json::Value)> = map
                .into_iter()
                .map(|(k, v)| (k, deep_sort(v)))
                .collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            // Rebuild as an ordered array of [key, value] pairs for deterministic comparison.
            serde_json::Value::Array(
                entries
                    .into_iter()
                    .map(|(k, v)| serde_json::json!([k, v]))
                    .collect(),
            )
        }
        serde_json::Value::Array(arr) => {
            // Deep-sort each element first, then sort the array by serialized form.
            let mut sorted: Vec<serde_json::Value> =
                arr.into_iter().map(deep_sort).collect();
            sorted.sort_by(|a, b| {
                serde_json::to_string(a)
                    .unwrap_or_default()
                    .cmp(&serde_json::to_string(b).unwrap_or_default())
            });
            serde_json::Value::Array(sorted)
        }
        other => other,
    }
}

fn format_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|p| format!("  {}", p.display()))
        .collect::<Vec<_>>()
        .join("\n")
}
