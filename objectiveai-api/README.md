# ObjectiveAI API Server

[![Crates.io](https://img.shields.io/crates/v/objectiveai-api.svg)](https://crates.io/crates/objectiveai-api)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Score everything. Rank everything. Simulate anyone.**

A self-hostable API server for [ObjectiveAI](https://objective-ai.io) - run the full ObjectiveAI platform locally or use the library to build your own custom server.

[Website](https://objective-ai.io) | [API](https://api.objective-ai.io) | [GitHub](https://github.com/ObjectiveAI/objectiveai) | [Discord](https://discord.gg/gbNFHensby)

## Overview

This crate provides two ways to use the ObjectiveAI API:

1. **Run the server** - Start a local instance of the ObjectiveAI API
2. **Import as a library** - Build your own server with custom authentication, routing, or middleware

## Running Locally

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- An [OpenRouter](https://openrouter.ai/) API key (for LLM access)
- Optionally, an ObjectiveAI API key (for Profile Computation)

### Quick Start

```bash
# Clone the repository
git clone https://github.com/ObjectiveAI/objectiveai
cd objectiveai/objectiveai-api

# Create a .env file
cat > .env << EOF
OPENROUTER_API_KEY=sk-or-...
OBJECTIVEAI_API_KEY=oai-...  # Optional
EOF

# Run the server
cargo run --release
```

The server starts on `http://localhost:5000` by default.

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `OPENROUTER_API_KEY` | (required) | Your OpenRouter API key |
| `OBJECTIVEAI_API_KEY` | (optional) | ObjectiveAI API key for caching and remote Functions |
| `OBJECTIVEAI_API_BASE` | `https://api.objective-ai.io` | ObjectiveAI API base URL |
| `OPENROUTER_API_BASE` | `https://openrouter.ai/api/v1` | OpenRouter API base URL |
| `ADDRESS` | `0.0.0.0` | Server bind address |
| `PORT` | `5000` | Server port |
| `USER_AGENT` | (optional) | User agent for upstream requests |
| `HTTP_REFERER` | (optional) | HTTP referer for upstream requests |
| `X_TITLE` | (optional) | X-Title header for upstream requests |

#### Backoff Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `CHAT_COMPLETIONS_BACKOFF_INITIAL_INTERVAL` | `100` | Initial retry interval (ms) |
| `CHAT_COMPLETIONS_BACKOFF_MAX_INTERVAL` | `1000` | Maximum retry interval (ms) |
| `CHAT_COMPLETIONS_BACKOFF_MAX_ELAPSED_TIME` | `40000` | Maximum total retry time (ms) |
| `CHAT_COMPLETIONS_BACKOFF_MULTIPLIER` | `1.5` | Backoff multiplier |
| `CHAT_COMPLETIONS_BACKOFF_RANDOMIZATION_FACTOR` | `0.5` | Randomization factor |

## Using as a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
objectiveai-api = "0.1.0"
```

### Example: Custom Server

```rust
use objectiveai_api::{chat, ctx, vector, functions, ensemble, ensemble_llm};
use std::sync::Arc;

// Create your HTTP client
let http_client = reqwest::Client::new();

// Create the ObjectiveAI HTTP client
let objectiveai_client = Arc::new(objectiveai::HttpClient::new(
    http_client.clone(),
    Some("https://api.objective-ai.io".to_string()),
    Some("apk...".to_string()),
    None, None, None,
));

// Build the component stack
let ensemble_llm_fetcher = Arc::new(
    ensemble_llm::fetcher::CachingFetcher::new(Arc::new(
        ensemble_llm::fetcher::ObjectiveAiFetcher::new(objectiveai_client.clone()),
    )),
);

let chat_client = Arc::new(chat::completions::Client::new(
    ensemble_llm_fetcher.clone(),
    Arc::new(chat::completions::usage_handler::LogUsageHandler),
    // ... upstream client configuration
));

// Use in your own Axum/Actix/Warp routes
```

## Architecture

### Modules

| Module | Description |
|--------|-------------|
| `auth` | Authentication and API key management |
| `chat` | Chat completions with Ensemble LLMs |
| `vector` | Vector completions for scoring and ranking |
| `functions` | Function execution and Profile management |
| `ensemble` | Ensemble management and caching |
| `ensemble_llm` | Ensemble LLM management and caching |
| `ctx` | Request context for dependency injection |
| `error` | Error response handling |
| `util` | Utilities for streaming and indexing |

### Component Stack

```
Request
    │
    ▼
┌─────────────────────────────────────────────────┐
│  Functions Client                               │
│  - Executes Function pipelines                  │
│  - Handles Profile weights                      │
└─────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────┐
│  Vector Completions Client                      │
│  - Runs ensemble voting                         │
│  - Combines votes into scores                   │
└─────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────┐
│  Chat Completions Client                        │
│  - Sends prompts to individual LLMs             │
│  - Handles retries and backoff                  │
└─────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────┐
│  Upstream Client (OpenRouter)                   │
│  - Actual LLM API calls                         │
└─────────────────────────────────────────────────┘
```

### Customization Points

Each layer uses traits for dependency injection:

- **Fetchers** - Implement custom caching or data sources for Ensembles, Functions, Profiles
- **Usage Handlers** - Track usage, billing, or analytics
- **Context Extensions** - Add per-request state (authentication, BYOK keys, etc.)

## API Endpoints

### Chat Completions
- `POST /chat/completions` - Create chat completion

### Vector Completions
- `POST /vector/completions` - Create vector completion
- `POST /vector/completions/{id}` - Get completion votes
- `POST /vector/completions/cache` - Get cached vote

### Functions
- `GET /functions` - List functions
- `GET /functions/{owner}/{repo}` - Get function
- `POST /functions/{owner}/{repo}` - Execute remote function with inline profile

### Profiles
- `GET /functions/profiles` - List profiles
- `GET /functions/profiles/{owner}/{repo}` - Get profile
- `POST /functions/{owner}/{repo}/profiles/{owner}/{repo}` - Execute remote function with remote profile
- `POST /functions/profiles/compute` - Train a profile

### Ensembles
- `GET /ensembles` - List ensembles
- `GET /ensembles/{id}` - Get ensemble

## License

MIT
