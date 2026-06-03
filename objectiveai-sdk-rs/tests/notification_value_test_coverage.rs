//! Asserts every concrete notification variant has a corresponding
//! round-trip test in `src/cli/output/tests.rs`. When a new variant
//! is added to `TypedNotificationValue` (or to the outer
//! `NotificationValue`'s non-`Typed` variants), this test fails
//! until a matching `nv_<snake_case_variant>_roundtrip` (or
//! `nv_<snake_case_variant>_<sub>_roundtrip` for variants that
//! split across sub-cases) lands too.
//!
//! The outer `NotificationValue` is a two-variant pass-through
//! (`Typed(TypedNotificationValue)` + `Other(Map)`). The original
//! coverage guarantee was per-payload-type; after the two-layer
//! refactor it lives on `TypedNotificationValue`. `Typed` itself is
//! skipped (no payload of its own) and only the typed sub-variants
//! plus the outer `Other` are required to have round-trip coverage.
//!
//! Pattern mirrors `tests/arbitrary_with_coverage.rs` — read a known
//! source path, parse with `syn`, panic with a sorted list of all
//! missing variants so the failure surfaces every gap at once.

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use syn::Item;

/// PascalCase → snake_case, matching serde's `rename_all = "snake_case"`
/// for single-word and acronym-free variant names. This intentionally
/// stays simple — the round-trip tests already lock in the exact wire
/// form per variant, so this transform only has to produce the test-fn
/// suffix, not arbitrary wire strings.
fn to_snake_case(ident: &str) -> String {
    let mut out = String::new();
    for (i, c) in ident.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

#[test]
fn every_notification_value_variant_has_a_roundtrip_test() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let nv_path = Path::new(manifest_dir)
        .join("src/cli/output/notification/notification_value.rs");
    let tests_path = Path::new(manifest_dir).join("src/cli/output/tests.rs");

    let nv_src =
        fs::read_to_string(&nv_path).expect("read notification_value.rs");
    let nv_file =
        syn::parse_file(&nv_src).expect("parse notification_value.rs");

    let mut variants: Vec<String> = Vec::new();
    for item in &nv_file.items {
        let Item::Enum(e) = item else { continue };
        match e.ident.to_string().as_str() {
            "NotificationValue" => {
                // Outer pass-through enum. `Typed` is unpacked below;
                // every other variant (`Other`, future siblings)
                // needs its own `nv_*_roundtrip`.
                for v in &e.variants {
                    let ident = v.ident.to_string();
                    if ident != "Typed" {
                        variants.push(ident);
                    }
                }
            }
            "TypedNotificationValue" => {
                for v in &e.variants {
                    variants.push(v.ident.to_string());
                }
            }
            _ => {}
        }
    }
    assert!(
        !variants.is_empty(),
        "neither NotificationValue nor TypedNotificationValue found in notification_value.rs",
    );

    let tests_src = fs::read_to_string(&tests_path).expect("read tests.rs");
    let tests_file = syn::parse_file(&tests_src).expect("parse tests.rs");

    let test_fn_names: Vec<String> = tests_file
        .items
        .iter()
        .filter_map(|item| {
            if let Item::Fn(f) = item {
                let has_test_attr = f.attrs.iter().any(|a| {
                    // Matches `#[test]`, `#[tokio::test]`, and any
                    // `…::test` path-suffixed attribute.
                    a.path()
                        .segments
                        .last()
                        .map_or(false, |s| s.ident == "test")
                });
                if has_test_attr {
                    return Some(f.sig.ident.to_string());
                }
            }
            None
        })
        .collect();

    // Longest-snake-first claim order: stops e.g. Agent's `nv_agent_`
    // prefix from stealing `nv_agent_items_*` tests that belong to
    // AgentItems. Each test fn is consumed by the most specific
    // (longest) matching variant; a variant with no remaining tests
    // matching its prefix is genuinely uncovered.
    let mut variant_order: Vec<&String> = variants.iter().collect();
    variant_order.sort_by_key(|v| std::cmp::Reverse(to_snake_case(v).len()));

    let mut consumed: HashSet<String> = HashSet::new();
    let mut uncovered: Vec<&String> = Vec::new();
    for variant in &variant_order {
        let prefix = format!("nv_{}_", to_snake_case(variant));
        let mut any = false;
        for test in &test_fn_names {
            if !consumed.contains(test) && test.starts_with(&prefix) {
                consumed.insert(test.clone());
                any = true;
            }
        }
        if !any {
            uncovered.push(*variant);
        }
    }

    if !uncovered.is_empty() {
        let mut lines: Vec<String> = uncovered
            .iter()
            .map(|variant| {
                let snake = to_snake_case(variant);
                format!(
                    "NotificationValue::{variant} has no round-trip test \
                     — add `nv_{snake}_roundtrip` (or `nv_{snake}_<sub>_roundtrip` \
                     for variants with sub-cases) in src/cli/output/tests.rs",
                )
            })
            .collect();
        lines.sort();
        panic!(
            "{} NotificationValue variant(s) lack a round-trip test:\n\n{}\n",
            lines.len(),
            lines.join("\n\n"),
        );
    }
}
