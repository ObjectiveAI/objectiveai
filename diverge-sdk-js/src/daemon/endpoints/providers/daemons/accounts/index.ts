// The accounts on a daemon record: what this daemon holds on another
// daemon, each by a name of the caller's choosing and the mode it
// authenticates there in. Added, rotated and forgotten one at a time.

export * as add from "./add/index.js";
export * as edit from "./edit/index.js";
export * as delete from "./delete/index.js";
