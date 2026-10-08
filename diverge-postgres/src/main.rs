//! The Postgres supervisor binary.
//!
//! In order: the root is found and the one `config.yaml` read — see
//! [`config`](diverge_sdk::config) — and the cluster the daemon's
//! block asks for installed, initialized, started, announced and kept
//! until it is told to stop — see [`serve`](diverge_postgres::serve). The runtime is built here
//! rather than attributed onto `main`, and a start that fails is the
//! one thing this prints, as the error returned.

use diverge_postgres::serve;
use diverge_sdk::config;

fn main() -> Result<(), serve::Error> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(serve::Error::Runtime)?;
    runtime.block_on(async {
        let root = config::root(std::env::args_os().skip(1)).await?;
        let config = config::load(&root).await?;
        serve::run(config.daemon.postgres, root).await
    })
}
