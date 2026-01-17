# ObjectiveAI JavaScript SDK

[![npm version](https://img.shields.io/npm/v/objectiveai.svg)](https://www.npmjs.com/package/objectiveai)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Score everything. Rank everything. Simulate anyone.**

The official JavaScript/TypeScript SDK for [ObjectiveAI](https://objective-ai.io) - a platform for remote Functions that score, rank, and simulate preferences using ensembles of LLMs.

[Website](https://objective-ai.io) | [GitHub](https://github.com/ObjectiveAI/objectiveai) | [Discord](https://discord.gg/gbNFHensby)

## Installation

```bash
npm install objectiveai
```

## Features

- **TypeScript definitions** with Zod schemas for runtime validation
- **ESM and CommonJS** support
- **Type-safe** request and response types for all API endpoints
- **WASM bindings** included for client-side validation and compilation

## Usage

```typescript
import { ... } from 'objectiveai';
```

The SDK provides comprehensive type definitions for:

- Ensemble LLMs and Ensembles
- Chat Completions
- Vector Completions
- Functions and Profiles
- Expressions (JMESPath)
- Rich content types (text, image, video, file, audio)
