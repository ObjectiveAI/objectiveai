# objectiveai-db-proxy

A Postgres-over-WebSocket conduit for ObjectiveAI plugin containers.

A container cannot reach the machine hosting it, so a plugin inside one
cannot dial the ObjectiveAI database directly. This binary is copied into
the container — the same way `objectiveai-mcp-laboratory` is copied into
laboratory containers — and bridges the two legs that *do* work:

- it **listens** on `127.0.0.1:14979` for ordinary Postgres clients, so a
  plugin connects with a plain connection string;
- it **listens** on `0.0.0.0:14980` for a WebSocket from the laboratory
  host, which dials in and relays to the real database.

Those four values are hardcoded, and there is **no configuration** — no
arguments, no environment, no `.env`. It is `podman exec`'d into an image
somebody else built, and an image that sets `ADDRESS` or `PORT` for its
own server has no business reconfiguring this.

Every Postgres connection is multiplexed over that one WebSocket, keyed
by a small numeric id. Payloads are opaque: pgwire is never parsed, so
TLS negotiation and protocol extensions pass through untouched.

The point is that a plugin — in any language, using the framework or not
— knows nothing about any of this. It sees a Postgres server on
localhost.

Part of the [ObjectiveAI](https://github.com/ObjectiveAI/objectiveai) monorepo.

## Links

- Homepage: <https://objectiveai.dev>
- Repository: <https://github.com/ObjectiveAI/objectiveai>
- Docs: <https://docs.rs/objectiveai-db-proxy>
