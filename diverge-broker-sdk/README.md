# diverge-broker-sdk

Wire types for the Diverge broker API.

This crate is the **normative artifact** of the broker specification,
the same way [`diverge-sdk`](../diverge-sdk-rs) is of
the provider's. The Rust types defined here *are* the protocol. Prose
documents the requirements a broker must satisfy; it does not define
the messages.

## Scope

Types only. No transport, no client, no server — both sides of the
protocol depend on this crate, as do tools that only inspect it, so it
carries no runtime concerns.

A broker answers **who somebody is**. Eventually it will also answer
who pays whom; nothing about payment is defined yet, and nothing here
should be read as anticipating a shape for it.

## Status

Bootstrapped and empty. The types land as the broker API is defined.

## License

MIT
