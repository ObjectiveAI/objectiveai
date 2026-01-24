# {ai} | ObjectiveAI

[![npm version](https://img.shields.io/npm/v/objectiveai.svg)](https://www.npmjs.com/package/objectiveai)
[![Crates.io](https://img.shields.io/crates/v/objectiveai.svg)](https://crates.io/crates/objectiveai)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Score everything. Rank everything. Simulate anyone.

ObjectiveAI is a platform for **Functions** - remote scoring/ranking pipelines powered by ensembles of LLMs. Define a Function, train it on your data, and call it from anywhere.

[Website](https://objective-ai.io) | [API](https://api.objective-ai.io) | [Discord](https://discord.gg/gbNFHensby) | [npm](https://www.npmjs.com/package/objectiveai) | [crates.io](https://crates.io/crates/objectiveai)

## Use cases

- Ranking search results, candidates, or recommendations
- Scoring content quality, code reviews, or support tickets
- Classifying documents or user intents
- Simulating how a specific person or group would decide

## Install

```bash
npm install objectiveai
```

```toml
[dependencies]
objectiveai = "0.1.1"
```

## How it works

### Ensembles

An **Ensemble** is a group of LLMs that vote together. Each LLM can have its own personality prompt, temperature, output mode, etc.

```json
{
  "llms": [
    {
      "model": "openai/gpt-4o",
      "output_mode": "json_schema",
      "prefix_messages": [
        { "role": "system", "content": "You are a rational skeptic. You select responses which are grounded in logic." }
      ],
      "count": 2
    },
    {
      "model": "anthropic/claude-sonnet-4-20250514",
      "output_mode": "tool_call",
      "suffix_messages": [
        { "role": "system", "content": "You are an emotional believer. You select responses that make you feel good." }
      ],
      "count": 1
    }
  ]
}
```

### Vector Completions

Give the ensemble a prompt and possible responses. Each LLM votes for what it thinks is the best response, and votes are combined with weights to produce scores.

```
Prompt: "What color is the sky?"
Responses: ["blue", "green", "red", "yellow"]

-> Scores: [0.85, 0.05, 0.05, 0.05]  (sums to 1)
```

#### Probabilistic Voting

LLMs are inherently probabilistic - the sampler makes the final discrete choice. ObjectiveAI bypasses the sampler entirely using **logprobs** to capture the model's full preference distribution.

Instead of asking "which is best?" and getting one answer, we extract the probability the model assigns to *each* option simultaneously. If the model is 70% confident in A and 30% in B, we capture that nuance rather than losing it to sampling.

```
Traditional: Model outputs "A" (loses the 30% signal for B)
ObjectiveAI: Model vote = [0.70, 0.30, 0.00, 0.00] (full distribution)
```

For large response sets exceeding logprobs limits, a prefix tree structure captures preferences in stages - the tree width matches logprobs count (typically 20), enabling voting over hundreds of options while preserving probability information at each level.

### Functions

**Functions** are the main interface. They're composable scoring pipelines hosted on GitHub as `function.json` files.

```
Input -> [Tasks] -> Score
```

Reference by `owner/repo`:

```
objectiveai/sentiment-scorer
```

### Profiles

ObjectiveAI doesn't fine-tune models - it learns **weights** over your ensemble.

Give it a dataset of inputs and expected outputs. It optimizes the weights to match, producing a **Profile** you can reuse. Profiles are GitHub-hosted as `profile.json`.

## Concepts

| Concept | Description |
|---------|-------------|
| **Ensemble LLM** | A configured LLM (model + settings). Content-addressed. |
| **Ensemble** | Group of Ensemble LLMs that vote together. No weights. |
| **Weights** | Per-model influence. Learned from data. |
| **Vector Completion** | Prompt + responses -> scores that sum to 1 |
| **Function** | Data in -> score out. GitHub-hosted. |
| **Profile** | Learned weights for a Function. GitHub-hosted. |

## Repo structure

```
objectiveai/
├── objectiveai-rs/           # Rust SDK (core crate)
├── objectiveai-api/          # API server (run locally or import as library)
├── objectiveai-rs-wasm-js/   # WASM bindings
├── objectiveai-js/           # TypeScript SDK
└── objectiveai-web/          # Web interface
```

## License

MIT
