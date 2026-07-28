import cn from "classnames";
import { useTabHarness } from "../lib/tabHarness";
import { useAgentsInstancesList } from "../hooks/useAgentsInstancesList";
import { HierarchyTree } from "../components/HierarchyTree";
import { LogoMark, Wordmark } from "../components/shared/Logo";

/** The agents home tab: watermark + hierarchy tree. Owns its own
 * agents-list connection — a tab component is self-contained beyond
 * the harness services. */
export default function AgentsTab() {
  const { transport, zoom } = useTabHarness();
  const agents = useAgentsInstancesList(transport);
  return (
    <div className={cn("relative", "flex-1", "min-h-0")}>
      {/* The brand mark: perfectly centered, always behind the body. */}
      <div
        className={cn(
          "absolute",
          "inset-0",
          "flex",
          "flex-col",
          "items-center",
          "justify-center",
          "gap-3",
          "pointer-events-none",
          "select-none",
        )}
      >
        <LogoMark className={cn("h-24", "w-auto", "text-info-dim/15")} />
        <Wordmark className={cn("w-[220px]", "h-auto", "text-info-dim/15")} />
      </div>
      {/* The body: the agent hierarchy tree, over the watermark. */}
      <HierarchyTree transport={transport} agents={agents} zoom={zoom} />
    </div>
  );
}
