// How many agents' logs the Agents map reads at once. Each card that shows
// what an agent is doing reads the tail of its log: watched while it works,
// once while it's idle. A map of many agents would otherwise open as many
// scopes; past the cap, a card waits its turn. Pure but for its one queue —
// tested.

export const TAILS_AT_ONCE = 12;

export type Turns = {
  /** Wait for a turn; the answer lets it go. Letting go twice, or after leaving the queue, does nothing. */
  take: () => { ready: Promise<void>; done: () => void };
  /** How many have a turn now, and how many wait. */
  counts: () => { open: number; waiting: number };
};

export function turns(cap: number): Turns {
  let open = 0;
  const waiting: (() => void)[] = [];
  const next = () => {
    while (open < cap && waiting.length) {
      open++;
      waiting.shift()!();
    }
  };
  return {
    take: () => {
      let had = false;
      let gone = false;
      let start: () => void = () => {};
      const ready = new Promise<void>((resolve) => {
        start = () => {
          had = true;
          resolve();
        };
      });
      waiting.push(start);
      next();
      return {
        ready,
        done: () => {
          if (gone) return;
          gone = true;
          if (had) open--;
          else waiting.splice(waiting.indexOf(start), 1);
          next();
        },
      };
    },
    counts: () => ({ open, waiting: waiting.length }),
  };
}

/** The map's one queue. */
export const tails = turns(TAILS_AT_ONCE);
