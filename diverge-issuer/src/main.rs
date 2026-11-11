//! The issuer binary.
//!
//! In order: the root is found and the one `issuer.yaml` read — see
//! [`config`](diverge_issuer::config) — and the issuer run on it until
//! it is told to stop — see [`serve`](diverge_issuer::serve). The
//! runtime is built here rather than attributed onto `main`, and a
//! start that fails is the one thing this prints, as the error
//! returned.

use diverge_issuer::config;
use diverge_issuer::serve;

fn main() -> Result<(), serve::Error> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(serve::Error::Runtime)?;
    runtime.block_on(async {
        let root = config::root(std::env::args_os().skip(1)).await?;
        let config = config::load(&root).await?;
        serve::run(config, root).await
    })
}
