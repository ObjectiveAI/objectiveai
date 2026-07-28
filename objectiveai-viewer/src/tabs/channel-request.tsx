import cn from "classnames";
import { useEffect, useState } from "react";
import * as Collapsible from "@radix-ui/react-collapsible";
import { type TabComponentProps } from "../lib/tabHarness";
import { Markdown } from "../components/Markdown";
import { JsonBlock } from "../components/shared/JsonBlock";
import { type DaemonChannelListenerChannelOffer } from "@objectiveai/sdk";
import {
  channelRequestAccept,
  channelRequestInstall,
  channelRequestStatus,
  tabsCloseSelf,
  type OfferStatus,
} from "../lib/tabs";

/** The wire ChannelOffer, verbatim — this tab's `arguments` (the
 * generated SDK type: caller identity FLATTENED to top-level fields;
 * its plugin trio is the publishing plugin). */
type ChannelOffer = DaemonChannelListenerChannelOffer;

/** One incoming channel offer, opened as a tab by the shell's
 * resident /channels listener. A render of its `arguments` plus the
 * STATUS-driven verb (queried from Rust on mount):
 * - `ready` — Accept hands off to Rust (POST the accept, spawn the
 *   plugin's handler component, kill this tab — success OR failure,
 *   this tab dies);
 * - `not_installed` — Install runs the daemon-build install with the
 *   two-step progress label ("building plugin…" → "installing
 *   plugin…"); success re-queries the status (→ Accept or
 *   unsupported), failure = Rust logs and kills this tab;
 * - `unsupported_key` / `no_plugin` — a dim explanation, Decline
 *   only.
 * Decline always closes the tab (there is no decline wire op;
 * ignoring IS declining). While a verb runs, everything locks and
 * the tab waits. */
export default function ChannelRequestTab({
  arguments: args,
}: TabComponentProps) {
  const offer = (args ?? {}) as ChannelOffer;
  const [status, setStatus] = useState<OfferStatus | "loading">("loading");
  const [busy, setBusy] = useState<
    null | "accepting" | "building" | "installing"
  >(null);

  useEffect(() => {
    let disposed = false;
    void channelRequestStatus().then((next) => {
      if (!disposed && next !== undefined) setStatus(next);
    });
    return () => {
      disposed = true;
    };
  }, []);

  const install = () => {
    setBusy("building");
    channelRequestInstall((step) => {
      setBusy(step.step === "installing" ? "installing" : "building");
    })
      .then(async () => {
        // Installed: what the tab shows next is the status's call
        // (accept, or unsupported key).
        const next = await channelRequestStatus();
        if (next !== undefined) setStatus(next);
        setBusy(null);
      })
      .catch(() => {
        // Rust logged and is closing this tab — just wait to die.
      });
  };
  const plugin =
    offer.plugin_owner !== undefined
      ? `${offer.plugin_owner}/${offer.plugin_name}/${offer.plugin_version}`
      : null;
  const senderFields: [string, string][] = [
    ["agent", offer.agent_instance_hierarchy ?? "anonymous"],
    ...(offer.agent_id !== undefined
      ? ([["agent id", offer.agent_id]] as [string, string][])
      : []),
    ...(offer.agent_full_id !== undefined
      ? ([["agent full id", offer.agent_full_id]] as [string, string][])
      : []),
    ...(offer.agent_remote !== undefined
      ? ([["remote", offer.agent_remote]] as [string, string][])
      : []),
    ...(offer.response_id !== undefined
      ? ([["response id", offer.response_id]] as [string, string][])
      : []),
    ...(offer.response_ids !== undefined
      ? ([["response ids", offer.response_ids]] as [string, string][])
      : []),
  ];

  return (
    <div className={cn("h-full", "min-h-0", "overflow-y-auto")}>
      <div className={cn("max-w-2xl", "mx-auto", "p-6", "flex", "flex-col", "gap-4")}>
        {/* Publisher: the plugin the offer came from. */}
        <div className={cn("flex", "items-baseline", "gap-3", "min-w-0")}>
          <span
            data-offer-plugin
            className={cn("font-mono", "text-sm", "text-copper-mid", "truncate")}
            title="publishing plugin"
          >
            {plugin ?? "anonymous"}
          </span>
          <span
            data-offer-key
            className={cn("font-mono", "text-xs", "text-info-dim", "truncate")}
            title="offer key"
          >
            {offer.key}
          </span>
          {offer.task === true && (
            <span
              title="fired by the task scheduler"
              className={cn(
                "shrink-0",
                "px-1",
                "rounded-xs",
                "bg-ground-surface",
                "text-copper-hot",
                "text-[9px]",
                "uppercase",
                "tracking-wider",
              )}
            >
              task
            </span>
          )}
        </div>

        {/* Sender identity. */}
        <div
          className={cn(
            "bg-ground-surface",
            "border",
            "border-node-border",
            "rounded-md",
            "p-2.5",
            "flex",
            "flex-col",
            "gap-1",
          )}
        >
          {senderFields.map(([label, value]) => (
            <div
              key={label}
              className={cn("flex", "items-baseline", "gap-2", "min-w-0", "text-xs")}
            >
              <span
                className={cn(
                  "shrink-0",
                  "text-info-dim",
                  "uppercase",
                  "text-[9px]",
                  "tracking-wider",
                )}
              >
                {label}
              </span>
              <span className={cn("font-mono", "text-info-mid", "break-all")}>
                {value}
              </span>
            </div>
          ))}
        </div>

        {/* The message — the human-readable ask. */}
        <div className={cn("text-sm", "text-info-bright")}>
          <Markdown>{offer.message ?? ""}</Markdown>
        </div>

        {/* The opaque details payload, collapsed by default. */}
        <Collapsible.Root defaultOpen={false}>
          <Collapsible.Trigger
            className={cn(
              "text-[10px]",
              "text-info-dim",
              "cursor-pointer",
              "hover:text-info-mid",
            )}
            aria-label="Toggle offer details"
          >
            details
          </Collapsible.Trigger>
          <Collapsible.Content>
            <JsonBlock
              value={offer.details}
              className={cn(
                "text-[11px]",
                "text-info-mid",
                "max-h-[300px]",
                "overflow-y-auto",
                "mt-1",
              )}
            />
          </Collapsible.Content>
        </Collapsible.Root>

        {/* The status-driven verb row. */}
        {status === "unsupported_key" && (
          <div
            data-offer-unsupported
            className={cn("text-[11px]", "text-info-dim", "font-mono")}
          >
            unsupported offer key — {offer.key} is not declared by{" "}
            {plugin ?? "the publishing plugin"}
          </div>
        )}
        {status === "no_plugin" && (
          <div
            data-offer-no-plugin
            className={cn("text-[11px]", "text-info-dim", "font-mono")}
          >
            no publishing plugin — nothing can handle this offer
          </div>
        )}
        <div className={cn("flex", "gap-2", "pt-2")}>
          {status === "ready" && (
            <button
              data-offer-accept
              disabled={busy !== null}
              onClick={() => {
                setBusy("accepting");
                // Rust POSTs the accept, spawns the handler tab, and
                // kills THIS tab — on failure it kills this tab too.
                void channelRequestAccept();
              }}
              className={cn(
                "px-4",
                "py-1.5",
                "rounded-md",
                "border",
                "border-copper-mid",
                "text-copper-bright",
                "text-xs",
                "uppercase",
                "tracking-wider",
                busy !== null
                  ? cn("opacity-50", "cursor-wait")
                  : cn(
                      "cursor-pointer",
                      "hover:bg-ground-surface",
                      "hover:border-copper-bright",
                    ),
                "transition-colors",
              )}
            >
              {busy === "accepting" ? "accepting…" : "accept"}
            </button>
          )}
          {status === "not_installed" && (
            <button
              data-offer-install
              disabled={busy !== null}
              onClick={install}
              className={cn(
                "px-4",
                "py-1.5",
                "rounded-md",
                "border",
                "border-copper-mid",
                "text-copper-bright",
                "text-xs",
                "uppercase",
                "tracking-wider",
                busy !== null
                  ? cn("opacity-50", "cursor-wait")
                  : cn(
                      "cursor-pointer",
                      "hover:bg-ground-surface",
                      "hover:border-copper-bright",
                    ),
                "transition-colors",
              )}
            >
              {busy === "building"
                ? "building plugin…"
                : busy === "installing"
                  ? "installing plugin…"
                  : "install"}
            </button>
          )}
          <button
            data-offer-decline
            disabled={busy !== null}
            onClick={() => tabsCloseSelf()}
            className={cn(
              "px-4",
              "py-1.5",
              "rounded-md",
              "border",
              "border-node-border",
              "text-info-dim",
              "text-xs",
              "uppercase",
              "tracking-wider",
              busy !== null
                ? cn("opacity-50", "cursor-wait")
                : cn(
                    "cursor-pointer",
                    "hover:text-info-mid",
                    "hover:bg-ground-surface",
                    "hover:border-info-dim",
                  ),
              "transition-colors",
            )}
          >
            decline
          </button>
        </div>
      </div>
    </div>
  );
}
