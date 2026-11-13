// Pulses: a short flat stroke runs a link each time something passes
// along it, as the events list tells.

import type { daemon, shared } from "@diverge/sdk";
import type * as Phaser from "phaser";
import { asPoints, laneLength, type Links, span } from "./links";
import type { Nodes } from "./nodes";
import { ink } from "./palette";

/** The running stroke's length and width, in pixels. */
const STROKE = 24;
const WIDTH = 3;

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

/** The pulses: told every change of the events list, after the world that holds it is drawn. */
export type Pulses = { told(change: shared.listing.Change<Event>): void };

/**
 * Run pulses as the events list tells (`told`).
 *
 * The rules:
 *
 * 1. Every edge's count as the first world held it is where it starts;
 *    one first seen later starts at nothing. Only rises are pulses, so
 *    a list opened again sends nothing twice.
 * 2. Every rise sends that many pulses, no more than `CAP` along one
 *    edge at once, `SPACING` apart: a short stroke in ink, `STROKE`
 *    long, running from the sender to the receiver along its call's
 *    lane and its bow (`Links.laneOf`), flat — no glow — and gone as it
 *    arrives.
 * 3. A pulse reads its lane at every step, so it follows a node being
 *    dragged and a lane that moves over.
 * 4. An edge with a node not on the board, or whose lane is hidden,
 *    sends nothing.
 * 5. While the board is not the one shown (`awake` says no), a rise is
 *    counted and sends nothing, so a board asleep piles up no pulses
 *    and wakes to none of the old.
 */
export function pulses(scene: Phaser.Scene, nodes: Nodes, links: Links, baseline: Event[], awake: () => boolean): Pulses {
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
    const first = links.laneOf(fromKey, toKey, "call");
    if (!first) {
      flying.set(event.id, (flying.get(event.id) ?? 1) - 1);
      return;
    }
    const g = scene.add.graphics().setDepth(8);
    const length = laneLength(first);
    scene.tweens.addCounter({
      from: 0,
      to: 1,
      duration: Math.max(SHORTEST, (1000 * length) / SPEED),
      ease: "Sine.easeInOut",
      // Rule 3: the stroke's head runs the lane, its tail `STROKE` behind, both kept on the lane.
      onUpdate: (tween) => {
        g.clear();
        const s = links.laneOf(fromKey, toKey, "call");
        if (!s) return;
        const run = laneLength(s);
        if (run <= 0) return;
        const head = Math.min(run, (tween.getValue() ?? 0) * (run + STROKE));
        const tail = Math.max(0, head - STROKE);
        const piece = span(s, tail, head);
        if (piece.length < 2) return;
        g.lineStyle(WIDTH, ink(), 1);
        g.strokePoints(asPoints(piece), false, false);
      },
      onComplete: () => {
        flying.set(event.id, (flying.get(event.id) ?? 1) - 1);
        g.destroy();
      },
    });
  };
  return {
    told(change) {
      if (change.kind === "listed") return;
      const event = change.item;
      if (change.kind === "removed") {
        counts.delete(event.id);
        return;
      }
      // Rule 2.
      const rise = event.count - (counts.get(event.id) ?? 0);
      counts.set(event.id, event.count);
      // Rule 5.
      if (!awake()) return;
      for (let i = 0; i < Math.min(rise, CAP); i++) scene.time.delayedCall(i * SPACING, () => run(event));
    },
  };
}
