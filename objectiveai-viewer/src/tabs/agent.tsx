import cn from "classnames";
import { useTabHarness, type TabComponentProps } from "../lib/tabHarness";
import { ConversationView } from "../components/ConversationView";
import { AgentChat } from "../components/AgentChat";

/** One agent conversation, by its instance hierarchy
 * (`arguments: { aih }`). */
export default function AgentTab({ arguments: args }: TabComponentProps) {
  const { transport } = useTabHarness();
  const { aih } = (args ?? {}) as { aih?: string };
  return (
    <>
      <div className={cn("flex-1", "min-h-0")}>
        <ConversationView transport={transport} hierarchy={aih ?? ""} />
      </div>
      <AgentChat hierarchy={aih ?? ""} />
    </>
  );
}
