// MCP as this SDK carries it: the five exchanges an MCP server is
// once the transport is off it — list tools, list resources, call a
// tool, read a resource, and the notifications it sends on its own
// account — each a request and an answering frame, a tag byte the
// whole of the distinction between them, shared by every scope that
// carries them in either direction; the server's error, whole; and
// the attestation every request a container makes and every
// response a tool gives carries in its `_meta`, under one key. The
// MCP shapes themselves — content blocks, tool results, params —
// are the Model Context Protocol's own, from
// `@modelcontextprotocol/core`, used where the Rust uses rmcp. Every
// frame here fails to decode the three ways every frame does, as a
// `DecodeError`.

export * as attestation from "./attestation/index.js";
export * as call_tool from "./call_tool/index.js";
export * as list_resources from "./list_resources/index.js";
export * as list_tools from "./list_tools/index.js";
export * as notifications from "./notifications/index.js";
export * as read_resource from "./read_resource/index.js";
export * from "./error_data.js";
export * from "./meta.js";
