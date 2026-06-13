# objectiveai-cli

CLI for ObjectiveAI. Built with `clap` derive macros.

## Building & Testing

```bash
cargo build --package objectiveai-cli
cargo test --package objectiveai-cli
```

## Command-to-Filesystem Mapping

The CLI command tree maps directly to the filesystem:

```
objectiveai agents get             → src/agents/mod.rs (Commands::Get)
objectiveai agents list active     → src/agents/list/mod.rs (Commands::Active)
objectiveai agents list available  → src/agents/list/mod.rs (Commands::Available)
objectiveai agents config get      → src/agents/config/mod.rs (Commands::Get)
objectiveai functions executions create → src/functions/executions/create/mod.rs
```

**Rule:** Each command level is a directory with `mod.rs`. Leaf commands are variants in the `Commands` enum. Every module exposes `pub enum Commands` with a `handle()` method.

## Module Structure

```
src/
├── main.rs              # Entry point, top-level Commands enum, Output enum
├── config.rs            # Config I/O: read(), write(), format_value(), format_jq()
├── error.rs             # CLI Error enum (thiserror)
├── remote.rs            # RemotePathCommitOptional args, Remote enum
├── get.rs               # Reusable GetArgs, GetPairArgs
├── list.rs              # Reusable list helpers (favorites, single, all, pair_*)
├── favorite.rs          # Reusable AddFavorite, EditFavorite arg structs
├── python.rs            # Python execution harness (embedded WASI rustpython via wasmtime)
├── python_wasm.rs       # The embedded rustpython.wasm blob + its build-time hash
├── api/                 # API config (mode, remote, local, headers)
├── agents/              # Agent commands (get, list, config, favorites)
├── swarms/              # Swarm commands
├── functions/           # Function commands (get, list, config, favorites, executions, inventions, profiles)
└── viewer/              # Viewer config
```

## Command Patterns

### Pattern A: Passthrough (nesting)

```rust
pub mod config;
pub mod favorites;

#[derive(Subcommand)]
pub enum Commands {
    Config { #[command(subcommand)] command: config::Commands },
    Favorites { #[command(subcommand)] command: favorites::Commands },
}

impl Commands {
    pub fn handle(self) -> Result<crate::Output, crate::error::Error> {
        match self {
            Commands::Config { command } => command.handle(),
            Commands::Favorites { command } => command.handle(),
        }
    }
}
```

### Pattern B: Config Get/Set (leaf)

```rust
#[derive(Subcommand)]
pub enum Commands {
    Get,
    Set { value: String },
}

impl Commands {
    pub fn handle(self) -> Result<crate::Output, crate::error::Error> {
        let (client, mut config) = crate::config::read()?;
        match self {
            Commands::Get => Ok(crate::Output::ConfigGet(
                crate::config::format_value(&config.api().get_mode()),
            )),
            Commands::Set { value } => {
                config.api().set_mode(value);
                crate::config::write(&client, &config)?;
                Ok(crate::Output::ConfigSet)
            }
        }
    }
}
```

### Pattern C: API commands (async)

```rust
impl Commands {
    pub async fn handle(self) -> Result<crate::Output, crate::error::Error> {
        match self {
            Commands::Get { args } => {
                let path = args.resolve(get_favorites)?;
                crate::api::run(|http_client| async move {
                    let response = objectiveai::agent::get_agent(&http_client, path).await?;
                    Ok(serde_json::to_string(&response).unwrap())
                }, false).await
            }
        }
    }
}
```

### Pattern D: Favorites management

Uses reusable `crate::favorite::AddFavorite` / `EditFavorite` structs. Variants: `Get`, `Add`, `Del`, `Edit`.

## Naming Conventions

- **Modules**: snake_case matching command names (`agents/`, `functions/executions/`)
- **Command enums**: Always `pub enum Commands` with `#[derive(Subcommand)]`
- **Argument structs**: Descriptive PascalCase with `#[derive(Args)]` (e.g., `GetArgs`, `ProfileArgs`)
- **Handler methods**: Always `handle(self)` or `async fn handle(self)`
- **Return type**: Always `Result<crate::Output, crate::error::Error>`

## Output Enum

All commands return `Output`:

```rust
pub enum Output {
    ConfigGet(String),   // Config value → stdout
    ConfigSet,           // Silent success
    Api(String),         // API response JSON → stdout
}
```

`main.rs` matches on this and prints to stdout. No `println!` outside `main.rs`.

## Async vs Sync

- **Sync**: Config commands (read/write local files)
- **Async**: API commands (use `crate::api::run()` which manages HTTP client and viewer)

When a command is async, all its parent `handle()` methods must also be async.

## Reusable Components

- `crate::get::GetArgs` — resolves `--favorite` or `--remote/--owner/--repository/--commit` to a path
- `crate::list::Source` — enum for `favorites`, `filesystem`, `objectiveai`, `all`
- `crate::favorite::{AddFavorite, EditFavorite}` — shared CRUD args for favorites
- `crate::api::run()` — runs async closure with HTTP client, handles Local/Remote/Viewer modes
