# @diverge/sdk

The Diverge SDK for TypeScript. Every type of the daemon protocol is
a [zod](https://zod.dev) schema and the type inferred from it, and
every endpoint is a function that performs its exchange over a
handle. The package follows `diverge-sdk-rs` module for module and is
written against the protocol specification at
<https://protocol.diverge.network>.

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

The app's Rust side implements four commands, each a thin call on
its handle. Bytes cross as byte arrays; a channel message is one
whole wire frame, header included.

| command | arguments | answers |
|---|---|---|
| `wire_request` | `payload`, `responses: Channel`, `requests: Channel` | the scope number |
| `wire_channel_request` | `scope`, `payload`, `responses: Channel` | the channel number |
| `wire_channel_response` | `scope`, `channel`, `payload` | nothing |
| `wire_channel_response_finish` | `scope`, `channel` | nothing |

`responses` receives every frame answering the scope, ending with
its finish; `requests` receives every channel request the daemon
opens on the scope, which the app answers through the last two
commands. A channel request's `responses` receives that channel's
answers, ending with its finish.

## What is here

- `wire` — the frames, the encodings, the client half, and the proxy
  half.
- `shared` — what more than one endpoint is made of: the one error,
  the filetree and its fold, the listing and its fold.
- `daemon` — the daemon protocol: the exchange shapes under
  `client`, and the endpoints under `endpoints`, one folder each.
