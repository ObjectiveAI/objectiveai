// A provider, as the screens name it: the daemon's identity for it,
// and the name you gave it.

import type { daemon } from "@diverge/sdk";

export type Identity = daemon.endpoints.agents.logs.server.response.Identity;

/** The one string a provider's identity reduces to: its subject at its issuer. The key the app's machine names are kept by. */
export function identityKey(identity: Identity): string {
  return `${identity.subject} at ${identity.issuer}`;
}

/** The provider's name on screen: the one you gave it, else the daemon's. */
export function machineName(names: Record<string, string>, identity: Identity): string {
  const key = identityKey(identity);
  return names[key] ?? key;
}
