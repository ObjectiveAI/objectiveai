//! `list_accounts`: the accounts the tool holds.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::failure::Failure;
use crate::server::Call;
use crate::store;

/// The tool's name.
pub const NAME: &str = "list_accounts";

/// What the tool does, as its caller reads it.
pub const DESCRIPTION: &str = "List the accounts at identity.diverge.network this tool holds. Each is `<issuer>.<id>`: a GitHub or Google sign-in, which owns every subject that begins with it. \
Give an account's `issuer` and `id` to `login`. No token is answered. There is no way to add an account yet, so this may answer none.";

/// What `list_accounts` takes: nothing.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parameters {}

/// Every account kept under the tool's template, by issuer and then
/// id: `{"accounts": [{"account", "issuer", "id"}]}`. None is an
/// empty list, and no failure.
pub async fn call(call: Call<'_>, _parameters: Parameters) -> Result<Value, Failure> {
    let pool = call.state.store.pool(&call.state.client).await?;
    let accounts = store::accounts::list(pool, call.template).await?;
    Ok(json!({ "accounts": accounts }))
}
