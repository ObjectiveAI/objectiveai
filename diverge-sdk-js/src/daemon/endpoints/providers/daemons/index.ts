// The daemons reached through the providers: who is discoverable on
// them, and the direct messages sent through them. `discover` lists the
// identities discoverable through the connected providers, what a
// `daemons` record names a remote by; `listen` hears the messages sent
// to the daemon through them; `message` sends one through them. Each is
// judged by the providers grant, and reaches only the providers that
// grant reaches.

export * as discover from "./discover/index.js";
export * as listen from "./listen/index.js";
export * as message from "./message/index.js";
