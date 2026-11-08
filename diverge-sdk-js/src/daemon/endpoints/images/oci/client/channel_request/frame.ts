// The channels a caller opens on the daemon's oci scope: the
// provider oci scope's own, the same tags and the same bodies — `0` a
// manifest, `1` a blob, `2` the proof, `3` the cancel — re-exported
// here since the path says this scope's channel requests live here,
// and they do.

export type { Frame } from "../../../../../../provider/endpoints/images/oci/client/channel_request/frame.js";
export { encode } from "../../../../../../provider/endpoints/images/oci/client/channel_request/frame.js";
