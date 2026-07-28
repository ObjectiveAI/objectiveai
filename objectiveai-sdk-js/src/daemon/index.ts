// Hand-written barrel (NOT auto-generated): re-exports the generated
// daemon wire types (generatedIndex) plus the hand-written client
// half — the unified `Client` (address+signature+identity, regular
// fetch+SSE mode or viewer Tauri-proxy mode; IS the /execute
// executor), its per-endpoint materialized listeners, the transports
// (`sse` fetch-mode, `viewerStream` Tauri-proxy mode + the injected
// `ViewerTransport` shape), and the viewer-plugin bundle download.
// The JS mirror of the Rust SDK's `objectiveai_sdk::daemon` module.
// Kept header-free so install-zod preserves it instead of overwriting
// it with a stub.
export * from "./generatedIndex";
export * from "./client";
export * from "./mode";
export * from "./sse";
export * from "./viewerStream";
export * from "./viewerPlugin";
export * from "./commandListener";
export * from "./channelListener";
export * from "./agentsInstancesListListener";
export * from "./agentsInstancesListener";
export * from "./laboratoriesListListener";
export * from "./laboratoriesListener";
export * from "./fileTree";
