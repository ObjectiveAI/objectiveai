use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::Path;

fn extract_refs(content: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let pattern = "\"$ref\"";

    let mut search_from = 0;
    while let Some(pos) = content[search_from..].find(pattern) {
        let pos = search_from + pos + pattern.len();
        search_from = pos;

        // Skip whitespace and colon
        let rest = &content[pos..];
        let rest = rest.trim_start();
        if !rest.starts_with(':') {
            continue;
        }
        let rest = rest[1..].trim_start();

        // Expect opening quote
        if !rest.starts_with('"') {
            continue;
        }
        let rest = &rest[1..];

        // Find closing quote
        if let Some(end) = rest.find('"') {
            let ref_name = &rest[..end];
            // Only include simple names (no URIs, no paths)
            if !ref_name.contains('/')
                && !ref_name.contains('#')
                && !ref_name.is_empty()
            {
                refs.push(ref_name.to_string());
            }
        }
    }

    refs
}

fn main() {
    // In a published crate (downloaded from crates.io) the schemas live
    // inside the package at `./schemas/`. In monorepo development they
    // live in the sibling crate `../objectiveai-json-schema/`. Try the
    // packaged location first, then fall back to the sibling.
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let packaged = manifest_dir.join("schemas");
    let sibling = manifest_dir.join("..").join("objectiveai-json-schema");

    let schema_dir = if packaged.is_dir() { packaged } else { sibling };

    let schema_dir = schema_dir
        .canonicalize()
        .expect("schema directory must exist (either ./schemas/ or ../objectiveai-json-schema/)");

    // Collect all schema files: name -> (absolute_path, refs)
    let mut schemas: BTreeMap<String, (String, Vec<String>)> = BTreeMap::new();

    let mut entries: Vec<_> = fs::read_dir(&schema_dir)
        .expect("failed to read schema directory")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in &entries {
        let path = entry.path();
        let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
        let abs_path = path
            .canonicalize()
            .unwrap()
            .to_str()
            .unwrap()
            .replace('\\', "/");
        let content = fs::read_to_string(&path).unwrap();
        let refs = extract_refs(&content);

        // Tell Cargo to rerun if this file changes
        println!("cargo::rerun-if-changed={}", path.display());

        schemas.insert(stem, (abs_path, refs));
    }

    // Generate schema_lookup.rs
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir).join("schema_lookup.rs");
    let mut out = fs::File::create(&out_path).unwrap();

    // schema_content function
    writeln!(
        out,
        "pub fn schema_content(name: &str) -> Option<&'static str> {{"
    )
    .unwrap();
    writeln!(out, "    match name {{").unwrap();
    for (name, (abs_path, _)) in &schemas {
        writeln!(out, "        {name:?} => Some(include_str!({abs_path:?})),")
            .unwrap();
    }
    writeln!(out, "        _ => None,").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    // schema_refs function
    writeln!(
        out,
        "pub fn schema_refs(name: &str) -> &'static [&'static str] {{"
    )
    .unwrap();
    writeln!(out, "    match name {{").unwrap();
    for (name, (_, refs)) in &schemas {
        if refs.is_empty() {
            continue;
        }
        // Deduplicate refs while preserving order
        let mut seen = std::collections::HashSet::new();
        let deduped: Vec<&str> = refs
            .iter()
            .filter(|r| seen.insert(r.as_str()))
            .map(|r| r.as_str())
            .collect();
        let refs_str = deduped
            .iter()
            .map(|r| format!("{r:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(out, "        {name:?} => &[{refs_str}],").unwrap();
    }
    writeln!(out, "        _ => &[],").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    // tool_name function
    writeln!(
        out,
        "pub fn tool_name(name: &str) -> Option<&'static str> {{"
    )
    .unwrap();
    writeln!(out, "    match name {{").unwrap();
    for name in schemas.keys() {
        // Dotted form here is fine — `InventionTool::new_sync` normalises
        // `.` → `_` once, on construction.
        let tool_name = format!("Read{name}Schema");
        writeln!(out, "        {name:?} => Some({tool_name:?}),").unwrap();
    }
    writeln!(out, "        _ => None,").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    // tool_description function
    writeln!(
        out,
        "pub fn tool_description(name: &str) -> Option<&'static str> {{"
    )
    .unwrap();
    writeln!(out, "    match name {{").unwrap();
    for name in schemas.keys() {
        let tool_desc = format!("Read {name} Schema");
        writeln!(out, "        {name:?} => Some({tool_desc:?}),").unwrap();
    }
    writeln!(out, "        _ => None,").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out, "}}").unwrap();
}
