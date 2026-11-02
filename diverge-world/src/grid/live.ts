// The nodes alight: who is working, which account is connected, whose
// container is down, which account is away.

import type * as Phaser from "phaser";
import type { Live } from "../world/model";
import type { Nodes } from "./nodes";

/** One breath of a working node's halo, and how long a halo takes to go out, in milliseconds. */
const BREATH = 1200;
const FADE = 400;

/** How faint a node whose container is down, or an account that is not connected, stands. */
const DOWN = 0.4;

/** The live state, told each node's by its key. */
export type Lit = { set(key: string, live: Live | null, deployed?: boolean): void };

/**
 * Light the nodes.
 *
 * The rules:
 *
 * 1. An agent or a tool `active` — its loop running, a call or a read in
 *    flight — has its halo breathing.
 * 2. An account `connected` has its halo steady.
 * 3. Neither, the halo goes out.
 * 4. An agent or a tool whose container is not deployed, and an
 *    account that is not connected, stands faint (`DOWN`), its name
 *    with it.
 */
export function makeLit(scene: Phaser.Scene, nodes: Nodes): Lit {
  const breathing = new Map<string, Phaser.Tweens.Tween>();
  return {
    set(key, live, deployed) {
      const node = nodes.byKey.get(key);
      if (!node) return;
      breathing.get(key)?.stop();
      breathing.delete(key);
      scene.tweens.killTweensOf(node.halo);
      if (live?.on && live.kind === "active") {
        // Rule 1.
        node.halo.setAlpha(0.35);
        breathing.set(key, scene.tweens.add({ targets: node.halo, alpha: 1, scale: 1.15, duration: BREATH / 2, yoyo: true, repeat: -1, ease: "Sine.easeInOut" }));
      } else if (live?.on) {
        // Rule 2.
        node.halo.setScale(1).setAlpha(0.75);
      } else {
        // Rule 3.
        scene.tweens.add({ targets: node.halo, alpha: 0, scale: 1, duration: FADE });
      }
      // Rule 4.
      if (deployed !== undefined) {
        node.image.setAlpha(deployed ? 1 : DOWN);
        node.label.setAlpha(deployed ? 0.9 : DOWN);
      }
    },
  };
}
