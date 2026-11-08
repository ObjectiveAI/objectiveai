// Performing the daemon's oci scope: opening it, and — where it is
// held — the provider scope's own handle, since the channels are the
// same. Its own files are flattened into it.

export * from "./execute.js";
export type { BlobStream, End } from "../../../../../../provider/endpoints/images/oci/client/execute/execute_handle.js";
export { ExecuteHandle } from "../../../../../../provider/endpoints/images/oci/client/execute/execute_handle.js";
