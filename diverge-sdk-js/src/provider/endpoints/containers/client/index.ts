// The caller half of a container scope: reading the channels this
// side opens on one. A unary channel is one answer then the finish;
// a channel stream is answers to the finish. Both are the provider
// protocol's, and the daemon's `standalone_tools.connect` borrows
// them, since its channels are the same channels.

export * from "./error.js";
export * from "./unary.js";
export * from "./channel_stream.js";
