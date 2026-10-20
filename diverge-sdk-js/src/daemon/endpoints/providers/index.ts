// Providers: the two ways the daemon comes to be connected to one,
// and the daemons it reaches through them.
//
// An `outgoing` provider the daemon DIALS, presenting a credential of
// the client's; an `incoming` provider dials the daemon, which judges
// its credential by the ones the client has added. A list or a get of
// either reports the daemon's `Status` with the provider and its
// latest capacity. A `daemons` record is another daemon the caller
// holds accounts on, reached through a provider both are connected to.

export * from "./status.js";
export * as daemons from "./daemons/index.js";
export * as incoming from "./incoming/index.js";
export * as outgoing from "./outgoing/index.js";
