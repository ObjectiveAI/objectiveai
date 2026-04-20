# objectiveai-web

New web frontend for ObjectiveAI. Next.js App Router.

## Data fetching

Use the `objectiveai` npm package for ALL API communication. Create an `ObjectiveAI` client and use the exported functions from the SDK modules (e.g. `Functions.list(client)`, `Functions.retrieve(client, ...)`). Read the SDK source in `../objectiveai-js/src/` to understand available methods.

**Never use `fetch()` directly against the API.** No API proxy routes. The SDK handles auth, headers, and error handling.

## Local development

The API server can be run locally from `../objectiveai-api/`. Point the SDK client at localhost when developing:

```ts
const client = new ObjectiveAI({ apiBase: "http://localhost:8080" });
```

## Rules

- All API calls go through the SDK. No exceptions.
- Run `pnpm` commands from the workspace root.
- No design system or copy guidance lives in this file. That comes from the design brief.