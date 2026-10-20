// What this SDK adds to MCP: the attestation every request a
// container makes and every response a tool gives carries in its
// `_meta`, under one key — who is asking, and who answered. The
// MCP shapes themselves — content blocks, tool results, metadata —
// are the Model Context Protocol's own, from `@modelcontextprotocol/core`,
// used where the Rust uses rmcp.

export * as attestation from "./attestation/index.js";
export * from "./meta.js";
