# @diverge/sdk

The Diverge SDK for TypeScript. Every type of the daemon protocol is
a [zod](https://zod.dev) schema and the type inferred from it, and
every endpoint is a function that performs its exchange over a
handle. The package follows `diverge-sdk-rs` module for module and is
written against the protocol specification at
<https://protocol.diverge.network>.

## Install

```sh
pnpm add @diverge/sdk zod
```

zod is a dependency, not a peer, and it is named here all the same:
the package's schemas and the Model Context Protocol's — which come
from `@modelcontextprotocol/core` — compose only within one zod
instance, so a project that holds its own copy wants the same one.
ESM only; Node 22.18 or later, or any runtime with a global
`WebSocket`.

## Two ways to hold a daemon

A handle is where requests go. There are two makers, and nothing
above them tells the difference.

**Over the wire.** `wire.client.dial(address, key)` opens a
WebSocket to the daemon — `ws://<address>/` when the address carries
no scheme, port 14980 by default — presents the account's key as the
first frame, and returns a handle that speaks the protocol itself.
This is the handle for Node, Deno, Bun and any browser allowed to
open the socket.

**Through a proxy.** `wire.proxy.handle(transport)` returns a handle
whose connection lives on the other side of a Tauri boundary: the
app's Rust holds the authenticated connection and its own
`diverge_sdk::wire::client::Handle`, and this side asks it frame by
frame. The webview never holds an address or a credential. The
transport is two functions the app supplies from `@tauri-apps/api`:

```ts
import { Channel, invoke } from "@tauri-apps/api/core";
import { wire } from "@diverge/sdk";

const handle = wire.proxy.handle({
  invoke: (command, args) => invoke(command, args),
  channel: () => new Channel(),
});
```

The app's Rust side is the Rust SDK's `diverge_sdk::tauri_proxy`,
with the crate's `tauri-proxy` feature: the app manages one
`TauriProxy` and registers its six commands beside its own.

```rust
tauri::Builder::default()
    .manage(diverge_sdk::tauri_proxy::TauriProxy::new(address, key))
    .invoke_handler(tauri::generate_handler![
        diverge_sdk::tauri_proxy::wire_request,
        diverge_sdk::tauri_proxy::wire_channel_request,
        diverge_sdk::tauri_proxy::wire_channel_response,
        diverge_sdk::tauri_proxy::wire_channel_response_finish,
        diverge_sdk::tauri_proxy::wire_status,
        diverge_sdk::tauri_proxy::wire_reset,
    ])
```

The page dials and learns how the link stands with
`wire.proxy.status(transport)`, and dials again after the connection
ended with `wire.proxy.reset(transport)`. Above the handle, the two
makers behave the same: when the connection ends, every open stream
fails with `closed` and every later send with a `connection` send
error, until the page dials again. The contract the six commands
keep — every channel carries whole frames and then one empty
message, a refusal carries its kind — is stated in
`src/wire/proxy/commands.ts`.

| command | arguments | answers |
|---|---|---|
| `wire_request` | `payload`, `responses: Channel`, `requests: Channel` | the scope number |
| `wire_channel_request` | `scope`, `payload`, `responses: Channel` | the channel number |
| `wire_channel_response` | `scope`, `channel`, `payload` | nothing |
| `wire_channel_response_finish` | `scope`, `channel` | nothing |
| `wire_status` | none | a `WireStatus` |
| `wire_reset` | none | nothing |

## A first exchange

An endpoint is reached by importing it — `daemon.endpoints.<family>.<request>.client.execute` —
never by a string. The answer is the frame: `found`, `not_found`,
`forbidden` and the daemon's own `error` are values to match, since
each is something the daemon said; a thrown error is the absence of
an answer.

```ts
import { daemon, shared, wire } from "@diverge/sdk";

const handle = await wire.client.dial("127.0.0.1:" + daemon.DEFAULT_PORT, key);

// One answer.
const answer = await daemon.endpoints.accounts.get.client.execute(handle, { account: { issuer: "https://auth.example.com", subject: "ci" } });
if (typeof answer === "object" && "found" in answer) {
  console.log(answer.found.identity);
}

// A list held open: every account as `added`, then `listed`, then
// every change, folded into a `Listing` by the record's key.
const { list } = daemon.endpoints.accounts;
const accounts = new shared.listing.Listing(list.server.response.key);
const [stream, cancel] = await list.client.execute(handle, {});
for await (const frame of stream) {
  const change = list.server.response.change(frame);
  if (change !== null) {
    accounts.push(change);
  }
  if (frame === "listed") {
    await cancel.cancel();
  }
}
console.log(accounts.items.length);
```

A download streams chunks with their bytes; an upload answers the
content channels the daemon opens, from a source the caller
supplies — a `Uint8Array`, or an async iterable of them:

```ts
import { daemon } from "@diverge/sdk";

const volume = { provider: { issuer: "https://auth.example.com", subject: "provider-1" }, name: "data" };
const uploaded = await daemon.endpoints.volumes.upload.client.execute(
  handle,
  { kind: "directory", volume, path: ["site"], files: ["index.html", "css/main.css"] },
  (path) => (path === null ? null : new TextEncoder().encode(`<!-- ${path} -->`)),
);
console.log(uploaded); // "uploaded", or why not
```

## What is here

- `wire` — the frames, the encodings, the client half, and the proxy
  half.
- `shared` — what more than one endpoint is made of: the one error,
  the filetree and its fold, the listing and its fold, the relay,
  the canonical form a value is hashed in, the grants, and `mcp`:
  the five MCP exchanges as frames, the server's error whole, and
  the attestation this SDK puts in a `_meta`.
- `daemon` — the daemon protocol: the exchange shapes under
  `client`, and the ninety endpoints under `endpoints`, one folder
  each.
- `provider` — the provider protocol as far as the daemon's answers
  reach into it: a volume's mode and its write content answer, a
  provider's `info` and `capacity`, the chunks an agent's run
  streams, and the channel readers a connect borrows.

The MCP shapes themselves — content blocks, tool results, params,
notifications — are `@modelcontextprotocol/core`'s own schemas,
used where the Rust uses rmcp and never renamed.

## Status

Every endpoint of the daemon's table has its folder, and every
schema is checked against the JSON Schema the daemon's own CLI
prints. Accounts, roles, providers, volumes, the vault, the
database, the events and the file exchanges over a volume have run
live against a daemon from this package. Agents, standalone tools,
dependency tools and the connect have not yet: they are mirrored and
schema-checked, and their live run waits on a provider with an image
to run. The proxy handle has listed agents and templates live through the
viewer's Tauri side; the end-of-channel contract and the Rust SDK's
`tauri_proxy`, which the viewer now uses, are compiled and
type-checked but not yet run live.
