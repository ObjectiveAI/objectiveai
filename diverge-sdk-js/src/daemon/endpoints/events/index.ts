// Events: the graph of who sent what to whom through the daemon.
//
// An EVENT is one edge of that graph: a sender and a receiver. One
// event per sender and receiver, with a `count` that grows; an edge
// is never removed. What passed is not said: the receiver's kind
// implies it, a message for an agent and a call or a read for a tool.
// `list` is the one request.

export * from "./event.js";
export * as list from "./list/index.js";
