//! Asserts that no field in any public type uses `Option<WithExpression<Option<...>>>`.
//! This double-Option pattern is ambiguous for serialization — use
//! `WithExpression<Option<T>>` with `skip_serializing_if = "WithExpression::is_none"` instead.

use std::fs;
use std::path::Path;
use syn::{Fields, Item, Type, Visibility};
use walkdir::WalkDir;

fn type_to_string(ty: &Type) -> String {
    quote::quote!(#ty).to_string()
}

/// Returns true if `s` ends with `::target` or equals `target` exactly.
fn ends_with_ident(s: &str, target: &str) -> bool {
    s == target || s.ends_with(&format!("::{target}"))
}

fn contains_option_with_expression_option(type_str: &str) -> bool {
    // Collapse whitespace so `functions :: expression :: WithExpression`
    // becomes `functions::expression::WithExpression`.
    let collapsed: String =
        type_str.chars().filter(|c| !c.is_whitespace()).collect();

    // Tokenize on `<`, `>`, and `,` to get path segments between angle brackets.
    // Then look for a sequence: [...Option] < [...WithExpression] < [...Option...]
    let tokens: Vec<&str> = collapsed
        .split(|c: char| c == '<' || c == '>' || c == ',')
        .map(|s| s.trim_matches(|c: char| c.is_whitespace()))
        .filter(|s| !s.is_empty())
        .collect();

    for window in tokens.windows(3) {
        if ends_with_ident(window[0], "Option")
            && ends_with_ident(window[1], "WithExpression")
            && ends_with_ident(window[2], "Option")
        {
            return true;
        }
    }
    false
}

fn check_fields(
    fields: &Fields,
    type_name: &str,
    file: &str,
    errors: &mut Vec<String>,
) {
    let field_list: Vec<_> = match fields {
        Fields::Named(named) => named
            .named
            .iter()
            .map(|f| {
                (
                    f.ident.as_ref().map(|i| i.to_string()).unwrap_or_default(),
                    type_to_string(&f.ty),
                )
            })
            .collect(),
        Fields::Unnamed(unnamed) => unnamed
            .unnamed
            .iter()
            .enumerate()
            .map(|(i, f)| (format!("{i}"), type_to_string(&f.ty)))
            .collect(),
        Fields::Unit => Vec::new(),
    };

    for (field_name, type_str) in field_list {
        if contains_option_with_expression_option(&type_str) {
            errors.push(format!(
                "{type_name}.{field_name} in {file} uses Option<WithExpression<Option<...>>> — \
                 use WithExpression<Option<T>> with skip_serializing_if instead"
            ));
        }
    }
}

#[test]
fn no_option_with_expression_option() {
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

        let source = fs::read_to_string(path).unwrap();
        let file = match syn::parse_file(&source) {
            Ok(f) => f,
            Err(_) => continue,
        };

        for item in &file.items {
            match item {
                Item::Struct(s) if matches!(s.vis, Visibility::Public(_)) => {
                    check_fields(
                        &s.fields,
                        &s.ident.to_string(),
                        &relative,
                        &mut errors,
                    );
                }
                Item::Enum(e) if matches!(e.vis, Visibility::Public(_)) => {
                    let name = e.ident.to_string();
                    for variant in &e.variants {
                        let qualified = format!("{name}::{}", variant.ident);
                        check_fields(
                            &variant.fields,
                            &qualified,
                            &relative,
                            &mut errors,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    if !errors.is_empty() {
        errors.sort();
        panic!(
            "Option<WithExpression<Option<...>>> is forbidden ({} violations):\n{}",
            errors.len(),
            errors.join("\n")
        );
    }
}
