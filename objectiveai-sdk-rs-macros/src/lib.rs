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
