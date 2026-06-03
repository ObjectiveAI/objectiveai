//! Ensures every `Option` or `WithExpression<Option<...>>` field with
//! `#[serde(skip_serializing_if = "...")]` also has
//! `#[schemars(extend("omitempty" = true))]`.
//!
//! Also ensures every `WithExpression<Option<...>>` field has
//! `#[serde(skip_serializing_if = "functions::expression::WithExpression::is_none")]`.
//!
//! Without this, JSON schema consumers (e.g. Go's `json.Marshal` with
//! `omitempty`) omit nil fields, but snapshot JSON includes them as `null`.
//! The `omitempty` extension lets code generators know the field is
//! conditionally omitted during serialization.

use std::fs;
use std::path::Path;
use syn::{Fields, GenericArgument, Item, PathArguments, Type, Visibility};
use walkdir::WalkDir;

/// Check if a type is `Option<...>`.
fn type_is_option(ty: &Type) -> bool {
    match ty {
        Type::Path(tp) => tp
            .path
            .segments
            .last()
            .map_or(false, |seg| seg.ident == "Option"),
        _ => false,
    }
}

/// Check if a type is `WithExpression<Option<...>>` (or any path ending in
/// `WithExpression` whose first generic argument is `Option<...>`).
fn type_is_with_expression_option(ty: &Type) -> bool {
    let Type::Path(tp) = ty else { return false };
    let Some(seg) = tp.path.segments.last() else {
        return false;
    };
    if seg.ident != "WithExpression" {
        return false;
    }
    let PathArguments::AngleBracketed(ref args) = seg.arguments else {
        return false;
    };
    args.args.first().is_some_and(|arg| {
        matches!(arg, GenericArgument::Type(inner) if type_is_option(inner))
    })
}

/// Check if attributes contain `#[serde(skip_serializing_if = "...")]`.
fn has_skip_serializing_if(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("serde") {
            let tokens = attr
                .meta
                .require_list()
                .ok()
                .map(|list| list.tokens.to_string());
            tokens.map_or(false, |t| t.contains("skip_serializing_if"))
        } else {
            false
        }
    })
}

/// Check if attributes contain `#[schemars(extend("omitempty" = true))]`.
fn has_schemars_omitempty(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("schemars") {
            let tokens = attr
                .meta
                .require_list()
                .ok()
                .map(|list| list.tokens.to_string());
            tokens.map_or(false, |t| t.contains("omitempty"))
        } else {
            false
        }
    })
}

/// Check if an item derives Serialize.
fn has_serialize_derive(attrs: &[syn::Attribute]) -> bool {
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
                        .map_or(false, |last| last.trim() == "Serialize")
                })
            })
        } else {
            false
        }
    })
}

/// Check if attributes contain a specific `skip_serializing_if` value.
fn has_skip_serializing_if_value(
    attrs: &[syn::Attribute],
    value: &str,
) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("serde") {
            let tokens = attr
                .meta
                .require_list()
                .ok()
                .map(|list| list.tokens.to_string());
            tokens.map_or(false, |t| {
                t.contains("skip_serializing_if") && t.contains(value)
            })
        } else {
            false
        }
    })
}

fn check_omitempty_fields(
    fields: &Fields,
    type_name: &str,
    relative: &str,
    errors: &mut Vec<String>,
) {
    if let Fields::Named(named) = fields {
        for field in &named.named {
            if (type_is_option(&field.ty)
                || type_is_with_expression_option(&field.ty))
                && has_skip_serializing_if(&field.attrs)
                && !has_schemars_omitempty(&field.attrs)
            {
                let field_name = field
                    .ident
                    .as_ref()
                    .map_or("?".to_string(), |i| i.to_string());
                errors.push(format!(
                    "{type_name}::{field_name} in {relative} has skip_serializing_if but is missing #[schemars(extend(\"omitempty\" = true))]"
                ));
            }
        }
    }
}

fn check_with_expression_option_skip(
    fields: &Fields,
    type_name: &str,
    relative: &str,
    errors: &mut Vec<String>,
) {
    if let Fields::Named(named) = fields {
        for field in &named.named {
            if type_is_with_expression_option(&field.ty)
                && !has_skip_serializing_if_value(
                    &field.attrs,
                    "WithExpression::is_none",
                )
            {
                let field_name = field
                    .ident
                    .as_ref()
                    .map_or("?".to_string(), |i| i.to_string());
                errors.push(format!(
                    "{type_name}::{field_name} in {relative} is WithExpression<Option<...>> but is missing #[serde(skip_serializing_if = \"functions::expression::WithExpression::is_none\")]"
                ));
            }
        }
    }
}

#[test]
fn all_optional_skip_fields_have_schemars_omitempty() {
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
                    let name = s.ident.to_string();
                    if !has_serialize_derive(&s.attrs) {
                        continue;
                    }
                    check_omitempty_fields(
                        &s.fields,
                        &name,
                        &relative,
                        &mut errors,
                    );
                }
                Item::Enum(e) if matches!(e.vis, Visibility::Public(_)) => {
                    let name = e.ident.to_string();
                    if !has_serialize_derive(&e.attrs) {
                        continue;
                    }
                    for variant in &e.variants {
                        let variant_name =
                            format!("{}::{}", name, variant.ident);
                        check_omitempty_fields(
                            &variant.fields,
                            &variant_name,
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
        panic!(
            "Option fields with skip_serializing_if missing #[schemars(extend(\"omitempty\" = true))] ({}):\n\n{}",
            errors.len(),
            errors.join("\n\n")
        );
    }
}

#[test]
fn all_with_expression_option_fields_have_skip_serializing_if() {
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
                    let name = s.ident.to_string();
                    if !has_serialize_derive(&s.attrs) {
                        continue;
                    }
                    check_with_expression_option_skip(
                        &s.fields,
                        &name,
                        &relative,
                        &mut errors,
                    );
                }
                Item::Enum(e) if matches!(e.vis, Visibility::Public(_)) => {
                    let name = e.ident.to_string();
                    if !has_serialize_derive(&e.attrs) {
                        continue;
                    }
                    for variant in &e.variants {
                        let variant_name =
                            format!("{}::{}", name, variant.ident);
                        check_with_expression_option_skip(
                            &variant.fields,
                            &variant_name,
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
        panic!(
            "WithExpression<Option<...>> fields missing #[serde(skip_serializing_if = \"functions::expression::WithExpression::is_none\")] ({}):\n\n{}",
            errors.len(),
            errors.join("\n\n")
        );
    }
}
