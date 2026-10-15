//! The vault: not served yet.

use bytes::Bytes;
use diverge_sdk::provider::client::Vault;

use super::Answerer;

/// What every vault ask is answered with until the vault is served.
const NOT_SERVED: &str = "the vault is not served yet";

/// Every vault ask fails, in the one sentence, until the vault's
/// wire is settled and served.
impl Vault for Answerer {
    async fn get(&self, _: &str) -> Result<Option<Bytes>, String> {
        Err(NOT_SERVED.to_string())
    }

    async fn set(&self, _: &str, _: Bytes) -> Result<(), String> {
        Err(NOT_SERVED.to_string())
    }

    async fn delete(&self, _: &str) -> Result<(), String> {
        Err(NOT_SERVED.to_string())
    }

    async fn lock(&self, _: &str, _: u32) -> Result<(), String> {
        Err(NOT_SERVED.to_string())
    }

    async fn unlock(&self, _: &str) -> Result<(), String> {
        Err(NOT_SERVED.to_string())
    }
}
