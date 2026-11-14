// A FUSE mount, from the proxy's side: what it asks of whoever serves
// it. The server asks, on channels it opens, so the asks are under
// `server`.

export * as server from "./server/index.js";
