# objectiveai-db-proxy

A Postgres-over-WebSocket conduit for ObjectiveAI plugin containers.

A container cannot reach the machine hosting it, so a plugin inside one
cannot dial the ObjectiveAI database directly. This binary is copied into
the container — the same way `objectiveai-mcp-laboratory` is copied into
laboratory containers — and bridges the two legs that *do* work:

- it **listens** on a fixed loopback TCP port for ordinary Postgres
  clients, so a plugin connects with a plain connection string;
- it **listens** on a second port for a WebSocket from the laboratory
  host, which dials in and relays to the real database.

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
