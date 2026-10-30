// Who sent a user part, as the sheet names it: never "user", always
// the sender.

import type { daemon, shared } from "@diverge/sdk";

type Creator = daemon.creator.Creator;
type Caller = shared.mcp.attestation.Caller;

/** A template's first eight characters and an index: how a nameless agent or tool is shown. */
const short = (template: string, index: number) => `${template.slice(0, 8)} #${index}`;

/** What a sender is called, and the colour class its label wears. */
export type Sender = { key: string; label: string; tone: "account" | "agent" | "tool" };

/**
 * The sender of a user part.
 *
 * The rules:
 *
 * 1. A client is its subject.
 * 2. An agent is its name, else its template's first eight characters
 *    and its index.
 * 3. A created tool is its name, else the same short form.
 * 4. A dependency tool is its image's name, found by its template's
 *    first eight characters in `images`, else those characters.
 * 5. The key is the sender's identity, never its label: two senders
 *    with one label are still two.
 */
export function sender(creator: Creator, images: Map<string, string>): Sender {
  switch (creator.type) {
    case "client":
      return { key: `client:${creator.identity.issuer}:${creator.identity.subject}`, label: creator.identity.subject, tone: "account" };
    case "agent":
      return { key: `agent:${creator.template}:${creator.index}`, label: creator.name ?? short(creator.template, creator.index), tone: "agent" };
    case "tool":
      if ("record" in creator) {
        const r = creator.record;
        return { key: `tool:${r.template}:${r.index}`, label: r.name ?? short(r.template, r.index), tone: "tool" };
      }
      return {
        key: `dependency:${creator.dependency.template}:${JSON.stringify(creator.dependency.owner)}`,
        label: images.get(creator.dependency.template.slice(0, 8)) ?? creator.dependency.template.slice(0, 8),
        tone: "tool",
      };
  }
}

/**
 * Who asked a tool, as the daemon attests it: an agent is its name,
 * else its template's first eight characters and its index; a client
 * its subject. The key is the caller's identity, as `sender`'s is, so
 * an agent asking and the same agent sending read as one.
 */
export function caller(who: Caller): Sender {
  switch (who.kind) {
    case "agent":
      return { key: `agent:${who.template}:${who.index}`, label: who.name ?? short(who.template, who.index), tone: "agent" };
    case "client":
      return { key: `client:${who.identity.issuer}:${who.identity.subject}`, label: who.identity.subject, tone: "account" };
  }
}

/**
 * A tool call's name as the sheet shows it: an MCP tool name whose
 * prefix — the part before the first `_` — is a dependency template's
 * first eight characters is shown as that tool's image name and the
 * rest, `courier · reply`; any other name as it is.
 */
export function toolName(name: string, images: Map<string, string>): string {
  const cut = name.indexOf("_");
  const image = cut > 0 ? images.get(name.slice(0, cut)) : undefined;
  return image ? `${image} · ${name.slice(cut + 1)}` : name;
}
