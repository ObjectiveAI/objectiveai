# Coding Agent Scratch

This folder is for testing ObjectiveAI SDK calls and exploring API behavior.

## Setup

```bash
cd coding-agent-scratch
pnpm install
```

## Running Tests

```bash
# Run the SDK test script
pnpm test

# Or directly with ts-node
pnpm exec ts-node test-sdk.ts
```

## Environment Variables

- `OBJECTIVEAI_API_KEY` - Your API key (optional, will use anonymous if not set)

## What to Test

- CORS behavior from browser environments
- Client-side SDK usage patterns
- API responses and error handling
- Credits retrieval for anonymous users
- Function execution with cache/RNG flags
