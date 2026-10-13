//! The Postgres supervisor binary.
//!
//! In order: the arguments are read — the root and the cluster's
//! settings, see [`arguments`](diverge_postgres::arguments) — and the
//! cluster installed, initialized, started, announced and kept until
//! it is told to stop — see [`serve`](diverge_postgres::serve). The
//! runtime is built here rather than attributed onto `main`, and a
//! start that fails is the one thing this prints, as the error
//! returned.

use diverge_postgres::arguments;
use diverge_postgres::serve;

fn main() -> Result<(), serve::Error> {
    let arguments = arguments::parse(std::env::args_os().skip(1))?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(serve::Error::Runtime)?;
    runtime.block_on(serve::run(arguments.root, arguments.max_connections))
}
