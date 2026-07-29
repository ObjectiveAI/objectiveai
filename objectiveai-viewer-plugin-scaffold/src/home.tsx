/**
 * The plugin's BASE tab — a regular viewer tab (no `channel_key`),
 * opened at viewer boot. Everything here is a deletable example,
 * mirroring the Rust scaffold's `scaffold_*_deleteme` tools.
 *
 * The wave button deliberately depends on `canvas-confetti` — a
 * package the HOST viewer does not ship — so working confetti proves
 * plugin viewer dependencies resolve from the plugin's OWN tree. The
 * browser button exercises the injected script and the mailbox
 * bridge (see `credential.tsx` for the full round trip).
 */

import { useState } from "react";
import confetti from "canvas-confetti";
import { openViewerTab } from "@objectiveai/sdk";
import { transport } from "./transport";

export default function ScaffoldHome() {
  const [waves, setWaves] = useState(0);
  const [error, setError] = useState<string | null>(null);

  return (
    <div className="scaffold-home">
      <div className="glyph">🎉</div>
      <h1>objectiveai-plugin-scaffold</h1>
      <p>
        The viewer half of the ObjectiveAI plugin scaffold. Its channel
        handler answers the Rust scaffold&apos;s{" "}
        <code>scaffold.credential</code> requests; the{" "}
        <code>scaffold-overlay-deleteme</code> script rides along in
        browser tabs it spawns.
      </p>
      <button
        type="button"
        onClick={() => {
          setWaves(waves + 1);
          void confetti({ particleCount: 60, spread: 60, origin: { y: 0.7 } });
        }}
      >
        wave {waves > 0 ? `(${waves})` : ""}
      </button>
      <button
        type="button"
        onClick={() => {
          setError(null);
          void (async () => {
            await openViewerTab(await transport(), {
              title: "browser (scaffold)",
              url: "https://example.com/",
              key: "scaffold-browser-deleteme",
              script: "scaffold-overlay-deleteme",
            });
          })().catch((e: unknown) => setError(String(e)));
        }}
      >
        open browser with overlay
      </button>
      {error !== null && <pre>{error}</pre>}
    </div>
  );
}
