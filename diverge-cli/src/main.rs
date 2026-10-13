//! The CLI binary.
//!
//! In order: the arguments are parsed — see
//! [`arguments`](diverge_cli::arguments) — the root found and the one
//! `cli.yaml` read — see [`config`](diverge_cli::config) — and the
//! command run — see [`cli`]. The runtime is built
//! here rather than attributed onto `main`, and a run that fails is
//! the one thing this prints, as the error returned.

use clap::Parser as _;
use diverge_cli::arguments::Arguments;
use diverge_cli::cli;

fn main() -> Result<(), cli::Error> {
    let arguments = Arguments::parse();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(cli::Error::Runtime)?;
    runtime.block_on(cli::run(arguments))
}
