# objectiveai-cli

Command-line interface for [ObjectiveAI](https://objectiveai.dev), the agentic collective-judgment harness.

Distributed two ways:

- **Library + binary** on [crates.io](https://crates.io/crates/objectiveai-cli): `cargo install objectiveai-cli`. Build from source; default features include the local viewer (requires the monorepo's `objectiveai-viewer` build artifacts).
- **Pre-compiled binary** via [GitHub Releases](https://github.com/ObjectiveAI/objectiveai/releases) (cross-platform; viewer bundled).

Most users want the GitHub Release. The crates.io publish is for downstream Rust crates (notably [`objectiveai-mcp`](https://crates.io/crates/objectiveai-mcp)) that link against the CLI as a library.

## Links

- Homepage: <https://objectiveai.dev>
- Repository: <https://github.com/ObjectiveAI/objectiveai>
- Docs: <https://docs.rs/objectiveai-cli>
- Releases: <https://github.com/ObjectiveAI/objectiveai/releases>
