import cn from "classnames";
import * as Collapsible from "@radix-ui/react-collapsible";
import { type TabComponentProps } from "../lib/tabHarness";
import { Markdown } from "../components/Markdown";
import { JsonBlock } from "../components/shared/JsonBlock";
import { tabsCloseSelf } from "../lib/tabs";

/** The wire ChannelOffer, verbatim — this tab's `arguments`. The
 * caller identity is FLATTENED to top-level fields; its plugin trio
 * is the publishing plugin. */
interface ChannelOffer {
  channel_id: string;
  agent_instance_hierarchy?: string;
  agent_id?: string;
  agent_full_id?: string;
  agent_remote?: string;
  response_id?: string;
  response_ids?: string;
  plugin_owner?: string;
  plugin_name?: string;
  plugin_version?: string;
  task?: boolean;
  key: string;
  details?: unknown;
  message: string;
}

/** One incoming channel offer, spawned into its own window by the
 * shell's resident /channels listener. A pure render of its
 * `arguments`: no transport, no listeners — Accept is a no-op until
 * the accept wire-up lands, Decline just closes the tab (there is no
 * decline wire op; ignoring IS declining, and this window is the
 * tab's sole occupant, so the window closes with it). */
export default function ChannelRequestTab({
  arguments: args,
}: TabComponentProps) {
  const offer = (args ?? {}) as ChannelOffer;
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

        {/* Accept / Decline. */}
        <div className={cn("flex", "gap-2", "pt-2")}>
          <button
            data-offer-accept
            onClick={() => {
              // TODO: POST /channels/{id}/accept through the resident
              // listener (S_owner rides its SSE connection).
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
              "cursor-pointer",
              "hover:bg-ground-surface",
              "hover:border-copper-bright",
              "transition-colors",
            )}
          >
            accept
          </button>
          <button
            data-offer-decline
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
              "cursor-pointer",
              "hover:text-info-mid",
              "hover:bg-ground-surface",
              "hover:border-info-dim",
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
