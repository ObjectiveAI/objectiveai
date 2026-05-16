//! Asserts that every field containing `u64`, `i64`, `Decimal`/`rust_decimal`,
//! or `IndexMap`/`indexmap` in its type signature has an `#[arbitrary(with = "...")]`
//! attribute. This ensures the `arbitrary` crate never tries to derive these types
//! natively (which would produce extreme values or fail to compile).

use std::fs;
use std::path::Path;
use syn::{Attribute, Fields, Item, Type};
use walkdir::WalkDir;

const NEEDS_CUSTOM_ARBITRARY: &[&str] = &["u64", "i64", "f64", "usize", "isize", "Decimal", "rust_decimal", "IndexMap", "indexmap"];

fn type_to_string(ty: &Type) -> String {
    quote::quote!(#ty).to_string()
}

fn type_needs_custom_arbitrary(type_str: &str) -> bool {
    type_str
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|t| !t.is_empty())
        .any(|token| NEEDS_CUSTOM_ARBITRARY.contains(&token))
}

fn has_arbitrary_with(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("arbitrary") {
            attr.meta
                .require_list()
                .ok()
                .map(|list| list.tokens.to_string())
                .map_or(false, |t| t.contains("with"))
        } else {
            false
        }
    })
}

fn has_arbitrary_derive(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("derive") {
            attr.meta
                .require_list()
                .ok()
                .map(|list| list.tokens.to_string())
                .map_or(false, |t| t.contains("Arbitrary"))
        } else {
            false
        }
    })
}

struct FieldCheck {
    type_name: String,
    field_name: String,
    type_str: String,
    file: String,
}

fn check_fields(fields: &Fields, type_name: &str, file: &str) -> Vec<FieldCheck> {
    let field_list: Vec<_> = match fields {
        Fields::Named(named) => named
            .named
            .iter()
            .map(|f| {
                (
                    f.ident.as_ref().map(|i| i.to_string()).unwrap_or_default(),
                    type_to_string(&f.ty),
                    has_arbitrary_with(&f.attrs),
                )
            })
            .collect(),
        Fields::Unnamed(unnamed) => unnamed
            .unnamed
            .iter()
            .enumerate()
            .map(|(i, f)| {
                (
                    format!("{i}"),
                    type_to_string(&f.ty),
                    has_arbitrary_with(&f.attrs),
                )
            })
            .collect(),
        Fields::Unit => Vec::new(),
    };

    field_list
        .into_iter()
        .filter(|(_, type_str, has_custom)| {
            type_needs_custom_arbitrary(type_str) && !has_custom
        })
        .map(|(name, type_str, _)| FieldCheck {
            type_name: type_name.to_string(),
            field_name: name,
            type_str,
            file: file.to_string(),
        })
        .collect()
}

#[test]
fn all_arbitrary_types_have_custom_with_on_special_fields() {
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
                Item::Struct(s) if has_arbitrary_derive(&s.attrs) => {
                    let name = s.ident.to_string();
                    for fc in check_fields(&s.fields, &name, &relative) {
                        errors.push(format!(
                            "{}.{} in {} has type `{}` which requires #[arbitrary(with = ...)]",
                            fc.type_name, fc.field_name, fc.file, fc.type_str
                        ));
                    }
                }
                Item::Enum(e) if has_arbitrary_derive(&e.attrs) => {
                    let name = e.ident.to_string();
                    for variant in &e.variants {
                        let variant_name = variant.ident.to_string();
                        let qualified = format!("{name}::{variant_name}");
                        for fc in check_fields(&variant.fields, &qualified, &relative) {
                            errors.push(format!(
                                "{}.{} in {} has type `{}` which requires #[arbitrary(with = ...)]",
                                fc.type_name, fc.field_name, fc.file, fc.type_str
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    if !errors.is_empty() {
        errors.sort();
        panic!(
            "Fields missing #[arbitrary(with = ...)] ({} errors):\n{}",
            errors.len(),
            errors.join("\n")
        );
    }
}
