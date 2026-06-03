//! Ensures every field/variant whose type involves `rust_decimal::Decimal`
//! (directly, or inside generic containers like `Vec<Decimal>`) has a
//! `#[serde(deserialize_with = "...")]` attribute.
//!
//! Without this, `Decimal`'s default `Deserialize` impl accepts strings via
//! `visit_str`, which causes untagged enum variants containing `Decimal` to
//! steal string values that belong to other variants (e.g. `Err(Value)`).

use std::fs;
use std::path::Path;
use syn::{Fields, Item, Type, Visibility};
use walkdir::WalkDir;

/// Recursively check if a type mentions "Decimal" anywhere.
fn type_contains_decimal(ty: &Type) -> bool {
    match ty {
        Type::Path(tp) => {
            for seg in &tp.path.segments {
                if seg.ident == "Decimal" {
                    return true;
                }
                // Check generic arguments (e.g. Vec<Decimal>, Option<Decimal>)
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments
                {
                    for arg in &args.args {
                        if let syn::GenericArgument::Type(inner_ty) = arg {
                            if type_contains_decimal(inner_ty) {
                                return true;
                            }
                        }
                    }
                }
            }
            false
        }
        Type::Reference(r) => type_contains_decimal(&r.elem),
        Type::Slice(s) => type_contains_decimal(&s.elem),
        Type::Array(a) => type_contains_decimal(&a.elem),
        Type::Tuple(t) => t.elems.iter().any(type_contains_decimal),
        Type::Paren(p) => type_contains_decimal(&p.elem),
        Type::Group(g) => type_contains_decimal(&g.elem),
        _ => false,
    }
}

/// Check if a list of field attributes contains `#[serde(deserialize_with = "...")]`.
fn has_serde_deserialize_with(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("serde") {
            let tokens = attr
                .meta
                .require_list()
                .ok()
                .map(|list| list.tokens.to_string());
            tokens.map_or(false, |t| t.contains("deserialize_with"))
        } else {
            false
        }
    })
}

/// Check if an item derives Deserialize.
fn has_deserialize_derive(attrs: &[syn::Attribute]) -> bool {
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
                        .map_or(false, |last| last.trim() == "Deserialize")
                })
            })
        } else {
            false
        }
    })
}

/// Check if an item has a manual Deserialize impl in the file.
fn has_manual_deserialize_impl(file: &syn::File, type_name: &str) -> bool {
    for item in &file.items {
        if let Item::Impl(impl_item) = item {
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
            if let Some((_, trait_path, _)) = &impl_item.trait_ {
                if let Some(last) = trait_path.segments.last() {
                    if last.ident == "Deserialize" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn module_prefix(path: &str) -> String {
    let inner = path.strip_prefix("src/").unwrap_or(path);
    let segments: Vec<&str> = inner.split('/').collect();
    let folders = &segments[..segments.len().saturating_sub(1)];
    if folders.is_empty() {
        String::new()
    } else {
        format!("{}.", folders.join("."))
    }
}

/// Check fields (named or unnamed) for Decimal types missing deserialize_with.
fn check_fields(
    fields: &Fields,
    type_name: &str,
    relative: &str,
    errors: &mut Vec<String>,
) {
    match fields {
        Fields::Named(named) => {
            for field in &named.named {
                if type_contains_decimal(&field.ty)
                    && !has_serde_deserialize_with(&field.attrs)
                {
                    let field_name = field
                        .ident
                        .as_ref()
                        .map_or("?".to_string(), |i| i.to_string());
                    let ty_str = quote::quote!(#field).to_string();
                    errors.push(format!(
                        "{type_name}::{field_name} in {relative} contains Decimal but is missing #[serde(deserialize_with = \"...\")]\n    field type: {ty_str}"
                    ));
                }
            }
        }
        Fields::Unnamed(unnamed) => {
            for (i, field) in unnamed.unnamed.iter().enumerate() {
                if type_contains_decimal(&field.ty)
                    && !has_serde_deserialize_with(&field.attrs)
                {
                    let ty_str = quote::quote!(#field).to_string();
                    errors.push(format!(
                        "{type_name}::{i} in {relative} contains Decimal but is missing #[serde(deserialize_with = \"...\")]\n    field type: {ty_str}"
                    ));
                }
            }
        }
        Fields::Unit => {}
    }
}

#[test]
fn all_decimal_fields_have_deserialize_with() {
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
                    // Only check types that derive Deserialize
                    if !has_deserialize_derive(&s.attrs)
                        && !has_manual_deserialize_impl(&file, &name)
                    {
                        continue;
                    }
                    check_fields(&s.fields, &name, &relative, &mut errors);
                }
                Item::Enum(e) if matches!(e.vis, Visibility::Public(_)) => {
                    let name = e.ident.to_string();
                    if !has_deserialize_derive(&e.attrs)
                        && !has_manual_deserialize_impl(&file, &name)
                    {
                        continue;
                    }
                    for variant in &e.variants {
                        let variant_name =
                            format!("{}::{}", name, variant.ident);
                        check_fields(
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
            "Decimal fields missing #[serde(deserialize_with)] ({}):\n\n{}",
            errors.len(),
            errors.join("\n\n")
        );
    }
}
