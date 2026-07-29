/**
 * The `scaffold.credential` channel handler — the viewer half of the
 * Rust scaffold's `scaffold_credential_call_deleteme` tool. When a
 * human Accepts that channel request, the viewer opens THIS component
 * as a tab; its `arguments` are `{ request, response }`: the
 * published offer (the URL wanting a credential rides
 * `request.details.url`) and the accept's response body —
 * `{ secret }`, the owner capability this handler presents on every
 * `channels/*` command it runs.
 *
 * Two ways to a credential, both replying
 * `{ "credential": "<value>" }` over the channel log (exactly the
 * shape the Rust tool reads):
 *
 * - type it into the form, or
 * - open the SITE in a browser tab with the scaffold's overlay script
 *   injected: the script's in-page form sends the value over the
 *   mailbox bridge, and this tab receives it on the same
 *   `subscribeViewerTab` lane any child tab would use. Bridge
 *   messages originate in a page this plugin does NOT own — they are
 *   UNTRUSTED input, which is why the value lands in the form for
 *   the human to confirm rather than auto-replying.
 */

import { useRef, useState } from "react";
import confetti from "canvas-confetti";
import {
  Client,
  channelsLogsReplyExecute,
  openViewerTab,
  subscribeViewerTab,
} from "@objectiveai/sdk";
import { transport } from "./transport";

const BROWSER_KEY = "scaffold-credential-browser-deleteme";

interface Offer {
  channel_id?: string;
  key?: string;
  message?: string;
  details?: { url?: string };
}

interface HandlerArguments {
  request?: Offer;
  response?: { secret?: string };
}

export default function CredentialHandler({
  arguments: args,
}: {
  arguments?: unknown;
}) {
  const { request, response } = (args ?? {}) as HandlerArguments;
  const url = request?.details?.url ?? "";
  const [credential, setCredential] = useState("");
  const [phase, setPhase] = useState<"idle" | "replied">("idle");
  const [error, setError] = useState<string | null>(null);
  const watching = useRef(false);

  const reply = async () => {
    setError(null);
    const channelId = request?.channel_id;
    const secret = response?.secret;
    if (channelId === undefined || secret === undefined) {
      setError("no channel to reply on (opened outside an accept?)");
      return;
    }
    const result = await channelsLogsReplyExecute(
      Client.viewer(await transport()),
      {
        channel_id: channelId,
        secret,
        content: { credential },
      },
    );
    if ("error" in result) {
      setError(JSON.stringify(result.error));
      return;
    }
    setPhase("replied");
    void confetti({ particleCount: 120, spread: 75, origin: { y: 0.7 } });
  };

  const openSite = async () => {
    setError(null);
    const t = await transport();
    await openViewerTab(t, {
      title: url,
      url,
      key: BROWSER_KEY,
      script: "scaffold-overlay-deleteme",
    });
    if (watching.current) return;
    watching.current = true;
    // Drain the browser child's mailbox until a credential arrives —
    // the overlay's `__objectiveai.send({ credential })` lands here.
    for (;;) {
      const messages = await subscribeViewerTab(t, BROWSER_KEY, 60_000);
      const found = messages.find(
        (m): m is { credential: string } =>
          typeof (m as { credential?: unknown })?.credential === "string",
      );
      if (found) {
        setCredential(found.credential);
        watching.current = false;
        return;
      }
    }
  };

  return (
    <div className="scaffold-credential">
      <div className="glyph">🔐</div>
      <h1>
        {phase === "replied" ? "credential sent" : "credential requested"}
      </h1>
      {request?.message !== undefined && <p>{request.message}</p>}
      <code>{url}</code>
      {phase === "idle" && (
        <>
          <input
            type="password"
            placeholder="paste or capture a credential"
            value={credential}
            onChange={(e) => setCredential(e.target.value)}
          />
          <div className="row">
            <button
              type="button"
              disabled={credential.length === 0}
              onClick={() => void reply().catch((e) => setError(String(e)))}
            >
              send credential
            </button>
            <button
              type="button"
              disabled={url.length === 0}
              onClick={() => void openSite().catch((e) => setError(String(e)))}
            >
              open the site
            </button>
          </div>
        </>
      )}
      {error !== null && <pre>{error}</pre>}
    </div>
  );
}
