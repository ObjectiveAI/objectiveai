// The daemons reached through the providers: who is listable on
// them, and the direct messages sent through them. `list` lists the
// identities listable through the connected providers, what a
// `daemons` record names a remote by; `listen` hears the messages sent
// to the daemon through them; `message` sends one through them. Each is
// judged by the providers_daemons grant, and reaches only the providers
// that grant reaches.

export * as list from "./list/index.js";
export * as listen from "./listen/index.js";
export * as message from "./message/index.js";
