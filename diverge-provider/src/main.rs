//! The provider binary.
//!
//! In order: the provider's directory is found, its file read and
//! checked, and the provider run on them until it is told to stop —
//! see [`config`](diverge_provider::config) and
//! [`serve`](diverge_provider::serve). The runtime is built
//! here rather than attributed onto `main`, and a start that fails
//! is the one thing this prints, as the error returned.

use diverge_provider::{config, serve};

fn main() -> Result<(), serve::Error> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(serve::Error::Runtime)?;
    runtime.block_on(async {
        let dir = config::dir(std::env::args_os().skip(1)).await?;
        let config = config::load(&dir).await?;
        serve::run(config, dir).await
    })
}
