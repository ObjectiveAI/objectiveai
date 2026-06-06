use proc_macro::TokenStream;

/// Marks a type's schema role in the Ref/Owned pattern.
///
/// Values: `Owned`, `Ref`, `RefOwnedEnum`
///
/// - `Owned`: The struct IS the canonical schema. Its `#[schemars(rename)]`
///   must equal the module path + type name with "Owned" stripped.
/// - `Ref`: The borrowed counterpart. Must NOT derive `JsonSchema`.
/// - `RefOwnedEnum`: The dispatch enum. Must NOT derive `JsonSchema`
///   (uses a manual impl delegating to the Owned variant).
#[proc_macro_attribute]
pub fn schema_override(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Exempts a type from the JSON-schema coverage tests entirely.
///
/// For serializable wire types that deliberately ship WITHOUT a
/// registered JSON schema. Canonical case: the root `ResponseItem`
/// aggregates — their schema's transitive expansion covers the whole
/// command tree, which downstream generated TypeScript cannot emit
/// declarations for (tsc's ~2M-char serialization cap, TS7056), and
/// nothing consumes the aggregate schemas.
///
/// The `json_schema_coverage` tests skip any struct/enum carrying
/// this attribute: no JsonSchema-derive requirement, no rename check,
/// no `json_schemas()` registration requirement.
#[proc_macro_attribute]
pub fn json_schema_ignore(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
