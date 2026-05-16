//! Diff `ApiCallSubType` against the `objectiveai-api` crate's
//! `Router::new()` chain. Every `.route(<path>, axum::routing::<method>(...))`
//! call in `objectiveai-api/src/run.rs` must have a corresponding
//! `ApiCallSubType` variant whose `#[serde(rename)]` is
//! `"<METHOD>_<PATH>"`, and vice versa.
//!
//! The test exists in the SDK crate (not the api crate) because the
//! enum it covers lives here; both crates are workspace members so the
//! sibling path is always present when `cargo test` runs.

use std::collections::BTreeSet;
use std::path::Path;

use syn::visit::Visit;
use syn::{Expr, ExprCall, ExprLit, ExprMethodCall, ExprPath, Item, Lit};

/// One (method, path) tuple. Lowercased method for stable ordering;
/// the canonical wire form is uppercase (see `Method::wire`).
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct Route {
    method: String, // uppercase: "POST", "GET", "DELETE"
    path: String,   // leading slash: "/agent/completions"
}

impl Route {
    fn rename(&self) -> String {
        format!("{}_{}", self.method, self.path)
    }
}

/// Walk an axum `Router::new()` chain like
/// `Router::new().route("/x", axum::routing::post(...)).route("/y", axum::routing::get(...))`
/// and collect `(METHOD, PATH)` for each `.route(...)` call.
struct RouteVisitor {
    routes: Vec<Route>,
}

impl<'ast> Visit<'ast> for RouteVisitor {
    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        // Recurse into receiver first so we descend the chain.
        syn::visit::visit_expr_method_call(self, call);

        if call.method != "route" {
            return;
        }
        if call.args.len() != 2 {
            return;
        }
        let path_arg = &call.args[0];
        let handler_arg = &call.args[1];

        let path = match string_literal(path_arg) {
            Some(s) => s,
            None => return,
        };
        let method = match extract_axum_routing_method(handler_arg) {
            Some(m) => m,
            None => return,
        };

        self.routes.push(Route { method, path });
    }
}

fn string_literal(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) => Some(s.value()),
        _ => None,
    }
}

/// Look for `axum::routing::<method>(...)` (`<method>` ∈ {get, post,
/// delete, put, patch, head, options, trace}); return its uppercase
/// form. Accepts both the fully-qualified call expression and a
/// `MethodCall { receiver: axum::routing::X, method: ... }` wrap that
/// closures inside `axum::routing::method(move |args| { ... })`
/// produce.
fn extract_axum_routing_method(expr: &Expr) -> Option<String> {
    let call = match expr {
        Expr::Call(c) => c,
        _ => return None,
    };
    let ExprCall { func, .. } = call;
    let path = match func.as_ref() {
        Expr::Path(ExprPath { path, .. }) => path,
        _ => return None,
    };
    let last = path.segments.last()?.ident.to_string();
    // axum's routing module lives at `axum::routing::<verb>`; the path
    // we receive may be the bare verb (use-imported) or the qualified
    // `axum::routing::<verb>`. The last segment is the verb either way.
    match last.as_str() {
        "get" | "post" | "delete" | "put" | "patch" | "head" | "options" | "trace" => {
            Some(last.to_uppercase())
        }
        _ => None,
    }
}

/// Parse the `objectiveai-api` router source and return its route set.
fn extract_api_routes() -> BTreeSet<Route> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let run_rs = Path::new(manifest_dir).join("../objectiveai-api/src/run.rs");
    let source = std::fs::read_to_string(&run_rs).unwrap_or_else(|e| {
        panic!("cannot read {}: {}", run_rs.display(), e);
    });
    let file = syn::parse_file(&source).unwrap_or_else(|e| {
        panic!("cannot parse {}: {}", run_rs.display(), e);
    });

    let mut visitor = RouteVisitor { routes: Vec::new() };
    syn::visit::visit_file(&mut visitor, &file);
    visitor.routes.into_iter().collect()
}

/// Parse the `viewer::api_call` module source and extract the
/// `#[serde(rename = "...")]` attribute on each `ApiCallSubType`
/// variant.
fn extract_subtype_renames() -> BTreeSet<Route> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let api_call_rs = Path::new(manifest_dir).join("src/viewer/api_call.rs");
    let source = std::fs::read_to_string(&api_call_rs).unwrap_or_else(|e| {
        panic!("cannot read {}: {}", api_call_rs.display(), e);
    });
    let file = syn::parse_file(&source).unwrap_or_else(|e| {
        panic!("cannot parse {}: {}", api_call_rs.display(), e);
    });

    let api_call_sub_type = file
        .items
        .iter()
        .find_map(|item| match item {
            Item::Enum(e) if e.ident == "ApiCallSubType" => Some(e),
            _ => None,
        })
        .expect("ApiCallSubType enum not found in src/viewer/api_call.rs");

    let mut routes = BTreeSet::new();
    for variant in &api_call_sub_type.variants {
        let rename = serde_rename(&variant.attrs).unwrap_or_else(|| {
            panic!(
                "variant {} is missing #[serde(rename = \"...\")]",
                variant.ident
            );
        });
        let (method, path) = rename.split_once('_').unwrap_or_else(|| {
            panic!(
                "variant {} rename {:?} does not match '<METHOD>_<PATH>'",
                variant.ident, rename
            );
        });
        routes.insert(Route {
            method: method.to_string(),
            path: path.to_string(),
        });
    }
    routes
}

fn serde_rename(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let list = attr.meta.require_list().ok()?;
        let tokens = list.tokens.to_string();
        let rest = tokens.strip_prefix("rename")?;
        let rest = rest.trim_start().strip_prefix('=')?;
        let rest = rest.trim_start().strip_prefix('"')?;
        let end = rest.find('"')?;
        return Some(rest[..end].to_string());
    }
    None
}

#[test]
fn api_call_sub_type_matches_api_router() {
    let api_routes = extract_api_routes();
    let enum_routes = extract_subtype_renames();

    let missing_in_enum: Vec<&Route> = api_routes.difference(&enum_routes).collect();
    let extra_in_enum: Vec<&Route> = enum_routes.difference(&api_routes).collect();

    if missing_in_enum.is_empty() && extra_in_enum.is_empty() {
        return;
    }

    let mut msg = String::from(
        "ApiCallSubType variants do not match objectiveai-api/src/run.rs routes\n",
    );
    if !missing_in_enum.is_empty() {
        msg.push_str("\nRoutes in api crate but missing from ApiCallSubType:\n");
        for r in &missing_in_enum {
            msg.push_str(&format!("  - {}  (add variant with #[serde(rename = \"{}\")])\n", r.rename(), r.rename()));
        }
    }
    if !extra_in_enum.is_empty() {
        msg.push_str("\nVariants in ApiCallSubType but missing from api crate:\n");
        for r in &extra_in_enum {
            msg.push_str(&format!("  - {}  (remove the variant or add the route)\n", r.rename()));
        }
    }
    panic!("{}", msg);
}
