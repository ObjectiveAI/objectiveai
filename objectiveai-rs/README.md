# ObjectiveAI Rust SDK

[![Crates.io](https://img.shields.io/crates/v/objectiveai.svg)](https://crates.io/crates/objectiveai)
[![Documentation](https://docs.rs/objectiveai/badge.svg)](https://docs.rs/objectiveai)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Score everything. Rank everything. Simulate anyone.**

The official Rust SDK for [ObjectiveAI](https://objective-ai.io) - a platform for remote Functions that score, rank, and simulate preferences using ensembles of LLMs.

[Website](https://objective-ai.io) | [Documentation](https://docs.rs/objectiveai) | [GitHub](https://github.com/ObjectiveAI/objectiveai) | [Discord](https://discord.gg/gbNFHensby)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
objectiveai = "0.1.1"
```

## Features

- **Data structures and validation** for Ensemble LLMs, Ensembles, Functions, and Profiles
- **Deterministic ID computation** using XXHash3-128 (content-addressed identities)
- **Client-side Function compilation** with JMESPath expression evaluation
- **HTTP client** with streaming support (optional, enabled by default)

### Feature Flags

- `http` (default) - Enables the HTTP client for API requests

To use as a pure data structure library without HTTP:

```toml
[dependencies]
objectiveai = { version = "0.1.1", default-features = false }
```
