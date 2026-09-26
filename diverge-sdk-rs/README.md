# diverge-sdk

The Diverge SDK: one crate for everything that crosses a Diverge wire.

- `wire` — one WebSocket, nine-byte frames, scopes and channels, the
  encode/decode contract, and the frame-level cores of both halves.
- `shared` — the shapes more than one endpoint is made of.
- `provider` — the provider protocol: its endpoints, the caller half
  and the provider half. The **normative artifact** of the provider
  specification at <https://protocol.diverge.network>: the Rust types
  here *are* the protocol, and where prose and crate disagree, the
  crate is correct.
- `daemon` — the daemon protocol, over the same wire.
- `container_proxy` — the proxy beside every container's program,
  from both sides: `outside`, the WebSocket a provider opens into it;
  `inside`, the loopback the program dials it on.

No feature flags: every half of every protocol is here for every
dependent. Each revision of the specification is the version of this
crate.
