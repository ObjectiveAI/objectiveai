use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Generic wire wrapper for every list-style notification:
/// `{"type":"notification","items":[...]}`. The element type varies
/// (e.g. `Items<ListItem>` for `agents list`, `Items<PairListItem>` for
/// `pairs list`, `Items<crate::filesystem::config::Favorite>` for
/// favorites listings, `Items<crate::filesystem::logs::ListItem>`
/// for log listings).
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[schemars(rename = "cli.output.notification.Items.{T}")]
pub struct Items<T> {
    pub items: Vec<T>,
}

/// One entry in a non-pair resource listing — either a favorite that
/// matches a remote resource or a resolved remote path. Untagged so the
/// wire shape is whichever underlying object matches.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.output.notification.ListItem")]
pub enum ListItem {
    #[schemars(title = "Favorite")]
    Favorite(crate::filesystem::config::Favorite),
    #[schemars(title = "Item")]
    Item(crate::RemotePath),
}

/// One entry in a function-profile pair listing.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.output.notification.PairListItem")]
pub enum PairListItem {
    #[schemars(title = "Favorite")]
    Favorite(crate::filesystem::config::PairFavorite),
    #[schemars(title = "Item")]
    Item(crate::functions::response::ListFunctionProfilePairItem),
}
