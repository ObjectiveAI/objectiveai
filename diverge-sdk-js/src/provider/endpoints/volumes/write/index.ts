// Writing one file into a volume: a caller names a volume and a
// destination in it; the provider asks for the content on a channel
// of its own, puts the file in place whole, and says so. Only the
// content answer is here — what a client streams back on the channel
// the provider opened — since it is what a daemon's upload is made
// of: the daemon asks a client for each file's content the same way,
// and the client answers with the same frames.

export * as client from "./client/index.js";
