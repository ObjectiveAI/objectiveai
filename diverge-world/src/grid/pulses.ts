// Pulses: a dot of light runs a link each time something passes along
// it, as the events list tells.

import { daemon } from "@diverge/sdk";
import type * as Phaser from "phaser";
import { handle } from "../lib/wire";
import type { Pair } from "../world/model";
import { keep, keeper, type Watching } from "../world/watch";
import { offsetOf, segment } from "./links";
import type { Nodes } from "./nodes";
import { NEON } from "./palette";
import { PULSE } from "./textures";

type Event = daemon.endpoints.events.Event;
type Party = daemon.endpoints.events.Party;

/** How fast a pulse runs, in pixels a second, and how long the shortest run takes, in milliseconds. */
const SPEED = 420;
const SHORTEST = 250;

/** The most pulses along one edge at once, and the time between those one rise sends. */
const CAP = 3;
const SPACING = 180;

/** A party's key, as `model` makes it. */
function partyKey(party: Party): string {
  switch (party.kind) {
    case "account":
      return `account:${party.issuer}:${party.subject}`;
    case "agent":
    case "standalone_tool":
    case "dependency_tool":
      return `${party.kind}:${party.template}:${party.index}`;
  }
}

/**
 * Keep the events list open and run pulses.
 *
 * The rules:
 *
 * 1. Every edge's count as the snapshot read it is where it starts; one
 *    first seen later starts at nothing. Only rises are pulses, so a
 *    list opened again sends nothing twice.
 * 2. Every rise sends that many pulses, no more than `CAP` along one
 *    edge at once, `SPACING` apart: a dot in the sender's neon from the
 *    sender to the receiver, on the line the link draws.
 * 3. A pulse reads where both nodes stand at every step, so it follows
 *    a node being dragged.
 * 4. An edge with a node not on the board sends nothing.
 */
export function pulses(scene: Phaser.Scene, nodes: Nodes, pairs: Pair[], baseline: Event[]): Watching {
  const byPair = new Map(pairs.filter((p) => !p.mount).map((p) => [`${p.a}\n${p.b}`, p]));
  // Rule 1.
  const counts = new Map(baseline.map((e) => [e.id, e.count]));
  const flying = new Map<number, number>();
  const run = (event: Event) => {
    const fromKey = partyKey(event.sender);
    const toKey = partyKey(event.receiver);
    const from = nodes.byKey.get(fromKey);
    const to = nodes.byKey.get(toKey);
    // Rule 4.
    if (!from || !to) return;
    if ((flying.get(event.id) ?? 0) >= CAP) return;
    flying.set(event.id, (flying.get(event.id) ?? 0) + 1);
    const pair = byPair.get(fromKey < toKey ? `${fromKey}\n${toKey}` : `${toKey}\n${fromKey}`);
    const offset = pair ? offsetOf(pair) : 0;
    const first = segment(from, to, offset);
    if (!first) {
      flying.set(event.id, (flying.get(event.id) ?? 1) - 1);
      return;
    }
    const dot = scene.add.image(first.a.x, first.a.y, PULSE).setTint(NEON[from.entity.kind]).setBlendMode("ADD").setDepth(8);
    const length = Math.hypot(first.b.x - first.a.x, first.b.y - first.a.y);
    scene.tweens.addCounter({
      from: 0,
      to: 1,
      duration: Math.max(SHORTEST, (1000 * length) / SPEED),
      ease: "Sine.easeInOut",
      // Rule 3.
      onUpdate: (tween) => {
        const s = segment(from, to, offset);
        if (!s) return;
        const t = tween.getValue() ?? 0;
        dot.setPosition(s.a.x + (s.b.x - s.a.x) * t, s.a.y + (s.b.y - s.a.y) * t);
      },
      onComplete: () => {
        flying.set(event.id, (flying.get(event.id) ?? 1) - 1);
        scene.tweens.add({ targets: dot, alpha: 0, scale: 2.2, duration: 220, onComplete: () => dot.destroy() });
      },
    });
  };
  const k = keeper();
  const events = daemon.endpoints.events.list;
  keep(
    { open: () => events.client.execute(handle, {}), change: events.server.response.change },
    (change) => {
      if (change.kind === "listed") return;
      const event = change.item;
      if (change.kind === "removed") {
        counts.delete(event.id);
        return;
      }
      // Rule 2.
      const rise = event.count - (counts.get(event.id) ?? 0);
      counts.set(event.id, event.count);
      for (let i = 0; i < Math.min(rise, CAP); i++) scene.time.delayedCall(i * SPACING, () => run(event));
    },
    k,
  );
  return k;
}
