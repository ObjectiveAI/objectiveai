// Over the remote daemons.
//
// The daemons this daemon reaches through its providers are its
// configuration's, read at its start: no request adds, edits or
// removes one. A grant over them reads them, connects a tool through
// one, or relays a request to one: on the wire a bare array of the
// `Action`s held, `{"daemons":["get","relay"]}`.

export * from "./action.js";
export * from "./permission.js";
