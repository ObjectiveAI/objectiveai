# objectiveai-provider-sdk

Wire types for the ObjectiveAI provider API.

This crate is the **normative artifact** of the provider
specification. The Rust types defined here *are* the protocol — the
JSON Schema corpus is generated from them, and the per-language SDKs
are generated from that. Prose documents the requirements a provider
must satisfy; it does not define the messages.

## Scope

Types only. No transport, no client, no server. That is deliberate:
both sides of the protocol depend on this crate, as do tools that only
inspect it, so it carries no runtime concerns.

Rules about *sequences* of messages — ordering, cardinality, what may
appear first or last — are not expressible in JSON Schema and live in
the prose specification instead, enforced by construction in the
provider frameworks.

## Status

Empty. The types land as the provider API is defined.

## License

MIT
