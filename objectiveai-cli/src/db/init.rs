//! Pool construction + migration runner. Bodies fill in at stage 2.

use std::path::Path;

use super::{Error, Pool};

/// Open the admin pool to the `postgres` system database, ensure
/// `objectiveai` exists, then open the application pool and run all
/// migrations.
pub async fn init(_config_base_dir: &Path) -> Result<Pool, Error> {
    unimplemented!("db::init — stage 2")
}
