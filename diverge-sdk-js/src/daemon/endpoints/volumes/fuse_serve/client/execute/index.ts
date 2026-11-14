// Performing the daemon's FUSE serve: opening it, and — where the
// volume is served — the provider scope's own handle, since the
// channels are the same. Its own files are flattened into it.

export * from "./execute.js";
export type { End, FiletreeStream } from "../../../../../../provider/endpoints/volumes/fuse_serve/client/execute/execute_handle.js";
export { ExecuteHandle } from "../../../../../../provider/endpoints/volumes/fuse_serve/client/execute/execute_handle.js";
