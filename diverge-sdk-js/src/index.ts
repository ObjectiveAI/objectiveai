// The Diverge SDK: one package for everything that crosses a Diverge
// wire, module for module the mirror of `diverge-sdk-rs`.
//
// `wire` is what every protocol is spoken over — the frames, the
// encodings, the client half that mints scopes and channels, and the
// proxy half that asks a Tauri app's Rust to do so. `shared` is the
// shapes more than one endpoint is made of. `daemon` is the daemon
// protocol: its exchange shapes and its endpoints.

export * as wire from "./wire/index.js";
export * as shared from "./shared/index.js";
export * as daemon from "./daemon/index.js";

/**
 * The most bytes one frame of content carries, anywhere in this
 * package — a container file read out or written in, a transfer's
 * chunk. The SENDER's rule alone: content larger than this leaves as
 * adjacent frames, and every receiver is chunk-naive — same content,
 * next frame, append — and never measures.
 */
export const CHUNK_SIZE = 2 * 1024 * 1024;
