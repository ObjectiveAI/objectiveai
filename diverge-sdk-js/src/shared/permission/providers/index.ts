// Over the daemon's providers.
//
// The daemon's providers are its configuration's, read at its start:
// no request adds, edits or removes one. A grant over them reads
// them, and nothing else: on the wire a bare array of the `Action`s
// held, `{"providers":["get","list"]}`.

export * from "./action.js";
export * from "./permission.js";
