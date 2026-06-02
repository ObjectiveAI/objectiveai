/// A CLI argument that accepts either a favorite/remote reference or inline JSON.
///
/// This is a generic building block for CLI args where a resource can be
/// specified either by reference (`favorite=name` or `remote=github,...`)
/// or as inline JSON.
///
/// # Usage
///
/// Since clap derive macros don't support generic `#[command(flatten)]` directly,
/// use the [`define_inline_or_ref!`] macro to generate concrete types.
///
/// # Resolution
///
/// - If `--<name>-inline <json>` is provided, deserializes the JSON into `T`.
/// - If `--<name> <ref>` is provided, resolves via favorites/remote to a
///   `RemotePathCommitOptional`, then wraps it with the provided constructor.

/// Generate a concrete `InlineOrRef`-style struct for a specific CLI argument name.
///
/// Creates a struct with two mutually exclusive args:
/// - `--$name <ref>` — a `FavoriteRef` (favorite or remote path)
/// - `--$name-inline <json>` — inline JSON
///
/// And a `resolve()` method that returns `$output_ty`.
///
/// # Example
///
/// ```ignore
/// define_inline_or_ref!(FunctionArg, "function", objectiveai_sdk::functions::FullInlineFunctionOrRemoteCommitOptional, Remote);
/// ```
///
/// This generates a `FunctionArg` struct with `--function` and `--function-inline` args,
/// resolving to `FullInlineFunctionOrRemoteCommitOptional`. The `Remote` variant is used
/// when constructing from a resolved `FavoriteRef`.
#[macro_export]
macro_rules! define_inline_or_ref {
    ($struct_name:ident, $arg_name:literal, $output_ty:ty, $remote_variant:ident) => {
        #[derive(clap::Args)]
        #[group(id = concat!($arg_name, "-group"), required = true, multiple = false)]
        pub struct $struct_name {
            #[arg(id = $arg_name, long = $arg_name, value_name = "REFERENCE", help = concat!("Reference (e.g. favorite=name or remote=github,owner=x,repository=y)"))]
            reference: Option<crate::favorite_ref::FavoriteRef>,
            #[arg(id = concat!($arg_name, "-inline"), long = concat!($arg_name, "-inline"), value_name = "INLINE", help = "Inline JSON definition")]
            inline: Option<String>,
        }

        impl $struct_name {
            pub async fn resolve<F, Fut>(
                self,
                get_favorites: F,
            ) -> Result<$output_ty, crate::error::Error>
            where
                F: FnOnce() -> Fut,
                Fut: std::future::Future<Output = Vec<crate::filesystem::config::Favorite>>,
            {
                if let Some(json) = self.inline {
                    let mut de = serde_json::Deserializer::from_str(&json);
                    return serde_path_to_error::deserialize(&mut de)
                        .map_err(crate::error::Error::InlineDeserialize);
                }
                if let Some(fav_ref) = self.reference {
                    let path = fav_ref.resolve(get_favorites).await?;
                    return Ok(<$output_ty>::$remote_variant(path));
                }
                unreachable!("clap group ensures one is set")
            }
        }
    };
}
