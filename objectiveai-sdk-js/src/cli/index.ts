// Hand-written barrel (NOT auto-generated): re-exports the generated
// cli types (generatedIndex). The daemon-endpoint clients that used
// to live here (broadcast/channel/agents/laboratories listeners, the
// SSE + viewer transports) moved to `src/daemon/` behind the unified
// `daemon.Client`. Kept header-free so install-zod preserves it
// instead of overwriting it with a stub.
export * from "./generatedIndex";
