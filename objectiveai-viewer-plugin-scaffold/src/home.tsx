/**
 * The plugin's BASE tab — a regular viewer tab (no `channel_key`),
 * opened at viewer boot. Everything here is a deletable example,
 * mirroring the Rust scaffold's `scaffold_*_deleteme` tools.
 *
 * The wave button deliberately depends on `canvas-confetti` — a
 * package the HOST viewer does not ship — so working confetti proves
 * plugin viewer dependencies resolve from the plugin's OWN tree.
 *
 * The browser button is the bridge demo without a channel in the way:
 * it spawns a browser tab with the capture script injected and prints
 * whatever that script sends back. The credential handler
 * (`credential.tsx`) is the same mechanism wired to a real channel.
 */

import { useRef, useState } from "react";
import confetti from "canvas-confetti";
import { openViewerTab, subscribeViewerTab } from "@objectiveai/sdk";
import { transport } from "./transport";

const BROWSER_KEY = "scaffold-browser-deleteme";
/** Same page as the credential handler, so the one capture script has
 * one target: pastebin's new paste, whose single body field it names
 * outright. */
const FORM_URL = "https://pastebin.com/";

export default function ScaffoldHome() {
  const [waves, setWaves] = useState(0);
  const [received, setReceived] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const watching = useRef(false);

  const openBrowser = async () => {
    const t = await transport();
    await openViewerTab(t, {
      title: "capture demo",
      url: FORM_URL,
      key: BROWSER_KEY,
      script: "scaffold-capture-deleteme",
    });
    // One drain loop for the tab's life — the mailbox is keyed by
    // (this tab, BROWSER_KEY), so re-opening rejoins the same one.
    if (watching.current) return;
    watching.current = true;
    for (;;) {
      const messages = await subscribeViewerTab(t, BROWSER_KEY, 60_000);
      if (messages.length > 0) {
        setReceived((prior) => [
          ...prior,
          ...messages.map((m) => JSON.stringify(m)),
        ]);
      }
    }
  };

  return (
    <div className="scaffold-home">
      <div className="glyph">🎉</div>
      <h1>objectiveai-plugin-scaffold</h1>
      <p>
        The viewer half of the ObjectiveAI plugin scaffold. Its channel
        handler answers the Rust scaffold&apos;s{" "}
        <code>scaffold.credential</code> requests; the{" "}
        <code>scaffold-capture-deleteme</code> script rides along in
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
          void openBrowser().catch((e: unknown) => setError(String(e)));
        }}
      >
        open a browser tab with the capture script
      </button>
      {received.length > 0 && (
        <pre>{received.join("\n")}</pre>
      )}
      {error !== null && <pre>{error}</pre>}
    </div>
  );
}
