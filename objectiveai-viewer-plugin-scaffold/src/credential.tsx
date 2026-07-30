/**
 * The `scaffold.credential` channel handler — the viewer half of the
 * Rust scaffold's `scaffold_credential_call_deleteme` tool. When a
 * human Accepts that channel request, the viewer opens THIS component
 * as a tab; its `arguments` are `{ request, response }`: the
 * published offer (the endpoint wanting a credential rides
 * `request.details.url`) and the accept's response body —
 * `{ secret }`, the owner capability this handler presents on every
 * `channels/*` command it runs.
 *
 * The whole flow is automatic and self-terminating:
 *
 * 1. On mount it IMMEDIATELY spawns a browser tab on a public paste
 *    form with the scaffold's capture script injected.
 * 2. It waits on that child's mailbox. The script's button sends
 *    `{ credential: <typed value> }`.
 * 3. Bridge messages come from a page this plugin does NOT own, so
 *    they are UNTRUSTED: the message must BE that shape and the
 *    string must be non-empty, or it is ignored and the wait
 *    continues.
 * 4. A valid one is written back over the channel with
 *    `channels logs reply` — the `{ "credential": "..." }` content
 *    the Rust tool reads.
 * 5. On a confirmed write it closes the browser tab and then itself.
 *    Nothing is left behind for the user to clean up.
 */

import { useEffect, useRef, useState } from "react";
import {
  Client,
  channelsLogsReplyExecute,
  closeViewerTab,
  openViewerTab,
  subscribeViewerTab,
} from "@objectiveai/sdk";
import { transport } from "./transport";

/** The mailbox name for the browser child — the only address this
 * tab can reach it (or close it) by. */
const BROWSER_KEY = "scaffold-browser-deleteme";

/** Pastebin's new-paste page: a public form with exactly ONE body
 * field, which is what makes the capture script's target unambiguous —
 * it harvests `#postform-text` by name rather than guessing at "the
 * first field" on a page with several. Nothing is ever submitted; the
 * page is only somewhere for a human to type. */
const FORM_URL = "https://pastebin.com/";

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

/** The one shape this handler accepts off the bridge: an object with
 * a non-empty string `credential`. Anything else is page noise. */
function credentialOf(message: unknown): string | null {
  if (typeof message !== "object" || message === null) return null;
  const value = (message as { credential?: unknown }).credential;
  return typeof value === "string" && value.length >= 1 ? value : null;
}

export default function CredentialHandler({
  arguments: args,
}: {
  arguments?: unknown;
}) {
  const { request, response } = (args ?? {}) as HandlerArguments;
  const [status, setStatus] = useState("opening the browser tab…");
  const [error, setError] = useState<string | null>(null);
  // React may mount an effect twice in development; the flow spawns a
  // browser and writes to a channel, so it runs exactly once.
  const started = useRef(false);

  useEffect(() => {
    if (started.current) return;
    started.current = true;

    const channelId = request?.channel_id;
    const secret = response?.secret;
    if (channelId === undefined || secret === undefined) {
      setError("no channel to reply on — opened outside an accept?");
      return;
    }

    void (async () => {
      const t = await transport();
      // Immediately, with no gesture: the browser tab IS this
      // handler's UI.
      await openViewerTab(t, {
        title: "capture a credential",
        url: FORM_URL,
        key: BROWSER_KEY,
        script: "scaffold-capture-deleteme",
      });
      setStatus("waiting for the value from the browser tab…");

      // Drain until something valid arrives. A subscribe returns as
      // soon as anything is pending and stops blocking once the child
      // closes, so the loop is driven by our own condition.
      let credential: string | null = null;
      while (credential === null) {
        const messages = await subscribeViewerTab(t, BROWSER_KEY, 60_000);
        for (const message of messages) {
          const found = credentialOf(message);
          if (found !== null) {
            credential = found;
            break;
          }
        }
      }

      setStatus("writing the credential to the channel…");
      const result = await channelsLogsReplyExecute(Client.viewer(t), {
        channel_id: channelId,
        secret,
        content: { credential },
      });
      if (result.type === "error") {
        setError(`reply failed: ${JSON.stringify(result.message)}`);
        return;
      }
      if (result.type === "channel_closed") {
        setError("the channel closed before the credential could be written");
        return;
      }

      // Written and acknowledged — take both tabs down. The child
      // first: closing self ends this component.
      setStatus("done");
      await closeViewerTab(t, { key: BROWSER_KEY });
      await closeViewerTab(t);
    })().catch((e: unknown) => setError(String(e)));
  }, [request, response]);

  return (
    <div className="scaffold-credential">
      <div className="glyph">🔐</div>
      <h1>{error === null ? status : "credential capture failed"}</h1>
      {request?.message !== undefined && <p>{request.message}</p>}
      {request?.details?.url !== undefined && (
        <code>{request.details.url}</code>
      )}
      {error !== null && <pre>{error}</pre>}
    </div>
  );
}
