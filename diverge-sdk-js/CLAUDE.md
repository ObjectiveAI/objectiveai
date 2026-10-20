# diverge-sdk-js

The Diverge SDK for TypeScript, published as `@diverge/sdk`. It is
the mirror of `diverge-sdk-rs`: the same module tree, the same names,
the same wire — hand-written against the protocol specification in
`diverge-provider-web`, which is what settles any disagreement.

## Build

From the repository root: `pnpm --filter @diverge/sdk typecheck` and
`pnpm --filter @diverge/sdk build` (tsup, one ESM bundle plus
declarations into `dist/`, which is not committed). Node 22.18 or
later: the global `WebSocket` is the only socket this package uses.

## A Rust type is a zod schema

Every Rust struct and enum that crosses the wire is one zod schema
and its inferred type, under ONE name — value and type namespaces,
the way a Rust name is both:

```ts
export const Frame = z.union([...]);
export type Frame = z.infer<typeof Frame>;
```

The doc comment is the Rust's, carried over as JSDoc on the const.
The schema states the JSON FORM of the type — what serde writes and
what the CLI prints — never the wire bytes:

- `Option<T>` is `.nullable()` where serde writes `null` and
  `.optional()` where it skips the member; a `Vec` is `z.array`; a
  `u64` is `z.number()` and never a `bigint`;
- a recursive type names its TS type first and annotates the schema
  `z.ZodType<T>` with `z.lazy`;
- an internally tagged enum (`tag = "x"`) is `z.discriminatedUnion("x", [...])`,
  each variant `z.object({ x: z.literal(...), ...members })`, never
  strict, since serde ignores what it does not know there;
- an externally tagged enum (`rename_all` alone) is a `z.union` of
  one-key `z.strictObject`s, a unit variant the bare `z.literal`;
- an untagged enum with `deny_unknown_fields` is a `z.union` of
  `z.strictObject`s in DECLARATION ORDER, since serde tries them in
  order and the strictness is what tells the shapes apart;
- `flatten` spreads the members into the parent (`.extend`, or a
  spread of `.shape`); a flattened tagged enum becomes a discriminated
  union of the parent extended per variant;
- a `transparent` newtype is the inner schema;
- a `Vec` or a map skipped when empty is `.optional()` too — absent
  means empty, and the schema states the wire, so never `.default([])`;
- `serde_json::Value` is `z.json()`; `u64` is `z.number().int().nonnegative()`;
  `DateTime<Utc>` is `z.iso.datetime({ offset: true })`; `IpAddr` is
  `z.union([z.ipv4(), z.ipv6()])`; `IndexMap<String, String>` is
  `z.record(z.string(), z.string())`;
- a generic Rust type is a FUNCTION of its parameter schemas:
  `within(only)`, `template(type)`;
- a rule across members (`overlay_disk` present exactly when the
  mode is `ephemeral`) is a `.refine` with a message naming it.

Where a Rust doc is stale against its code, the code wins and the
doc is not carried: the postgres action is `list`, not
`connections`; a connected tool reference is daemon, account and
tool, not a pair.

## Checking a schema against the Rust

The CLI prints the SDK's own JSON Schema for every request and
response: `diverge run <family> <request> request-schema` and
`response-schema`. A schema here is checked by feeding one JSON
example per variant to both — the zod schema, and Python
`jsonschema` against the CLI's `$defs` — and seeing both accept and
both reject the same values. Two things the CLI's schema cannot say:
`format: date-time` is not checked without `rfc3339-validator`, and
a template's flat `type` member is absent from the schema though
serde requires it on the wire.

## Module path is export path

The `src/` tree is the Rust crate's `src/` tree, one file per type,
and every directory's `index.ts` is the Rust `mod.rs`: `export * as
x from "./x/index.js"` for a directory, `export * from "./x.js"` for
a file flattened into its parent — the Rust's `mod x; pub use x::*;`.
A consumer writes `daemon.endpoints.accounts.get.client.execute(...)`
and `shared.filetree.response.Frame`, never a string naming either.

## The wire codec sits beside the schema

Where the Rust type implements `Encode` and `Decode`, the module
exports `encode(value): Uint8Array` and `decode(bytes): value`, built
from the module's own `TAG` constants and the helpers in
`wire/encode.ts`, `wire/decode.ts` and `wire/postcard.ts`. A frame's
tag table is stated once, in the module, as the Rust states it.

## An endpoint is a folder of functions

`daemon/endpoints/<family>/<request>/` holds `client/request.ts`
(the request schema, its `TAG`, its `encode`), `server/response.ts`
(the response schema and its `decode`) and `client/execute.ts`, whose
`execute(handle, request)` performs the exchange over one of
`daemon/client`'s shapes: `one_shot`, `stream` (with a `Cancel`),
`pending`. Every `execute` takes a `Handle` and never knows which
kind it was handed. There are no path strings and no registries: an
endpoint is reached by importing it.

## Two handles, one seam

`wire.client.dial(address, key)` makes a handle over a real
WebSocket: it speaks the nine-byte frames, mints scopes and channels,
and routes what comes back. `wire.proxy.handle(transport)` makes a
handle over a Tauri app's `invoke` and `Channel`, where the Rust side
holds the connection and this side asks it frame by frame; the
transport is structurally typed, so this package never imports
`@tauri-apps/api`. Above the handle nothing differs.

## House rules

- One topic per file; the file is named for the type it holds.
- Hand-written error classes, one per Rust error enum, with a `kind`
  discriminant and the cause. Nothing throws a bare string.
- No `ws`, no `@tauri-apps/api`, no `node:` imports in `src/`. The
  globals used are `WebSocket`, `TextEncoder`, `TextDecoder`.
- Dependencies are pinned exact, as the Rust crates are, and are
  bumped by looking up the latest version, never by hand.
- No tests. Verification is `typecheck`, `build`, and the product
  used live against a running daemon.
- Doc comments wrap near 72 columns, in the present tense, and say
  what a thing is and why it is shaped so.
