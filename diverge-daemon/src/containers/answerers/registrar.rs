//! The container registered: its dependencies deployed, its secrets
//! declared, taken and answered.

use diverge_sdk::provider::client::Registrar;
use diverge_sdk::shared::containers::dependencies::Template;
use diverge_sdk::shared::containers::secrets::{Declaration, Values};
use diverge_sdk::shared::error::Error;

use super::Answerer;
use crate::containers::deploy;
use crate::containers::secrets::{self, Secrets};

/// The one ask a provider makes before the container's id is out, the
/// id told here, answered in order:
///
/// 1. Every dependency deployed as [`deploy`](crate::containers::deploy)
///    states, where the program declared any; the first that cannot be
///    is the run's error. A tool container declares none, and one that
///    does is refused there.
/// 2. The program's declaration checked against the template's names
///    — [`secrets::declare`] — and the names it takes bound to vault
///    keys through the record's binding — [`secrets::bind`]; a
///    dependency's through the agent that spawns it, which deploy
///    already checked its template against.
/// 3. The first values taken — [`secrets::acquire`] — one slot held
///    per rotating name, under the record as holder; a key with no
///    value, or no free one, is the run's error.
/// 4. The secrets state stashed on the answerer, since no run exists
///    yet: the start moves it into the run, or releases what it holds
///    when the provider refuses the run after all.
///
/// Every error of 2 and 3 is of kind `secrets`, and names secrets and
/// keys, never a value.
impl Registrar for Answerer {
    async fn register(&self, id: String, dependencies: Vec<Template>, declaration: Declaration) -> Result<Values, Error> {
        if !dependencies.is_empty() {
            deploy::deploy(self, id, dependencies).await?;
        }
        secrets::declare(&self.declared, &declaration).map_err(|reason| error(&reason))?;
        let names: Vec<String> = declaration.keys().cloned().collect();
        let bound = secrets::bind(&names, &self.binding).map_err(|reason| error(&reason))?;
        let mut state = Secrets {
            declared: declaration,
            bound,
            held: None,
        };
        let values = secrets::acquire(&self.daemon, &mut state, &self.sender)
            .await
            .map_err(|failure| error(&failure.to_string()))?;
        *self.secrets.lock().await = Some(state);
        Ok(values)
    }
}

/// The run's error over its secrets.
fn error(reason: &str) -> Error {
    Error(serde_json::json!({
        "kind": "secrets",
        "error": reason,
    }))
}
