import { useState } from "react";
import { AgentMap } from "../components/AgentMap";
import { Button, Empty, Icon, Segmented } from "../components/ui";
import { useShared } from "../lib/context";
import { DEFAULT_LAYOUT, type Layout } from "../lib/map";
import { t } from "../strings";

/** The Agents tab: which of your agents are working, who made whom, what each is doing now, what waits on
 *  you, and what each has. Every card opens its conversation. */
export function AgentsMap() {
  const { agents, agentsState, tree, cards, open } = useShared();
  const [layout, setLayout] = useState<Layout>(DEFAULT_LAYOUT);
  const working = agents.filter((a) => a.active).length;
  const waiting = cards.filter((c) => agents.some((a) => a.key === c.agent)).length;
  return (
    <div className="map">
      <header className="map-head">
        <div className="map-titles">
          <h1>{t.map.title}</h1>
          <p className="muted small">
            {working} {t.map.working} · {waiting} {t.map.waitingOnYou}
          </p>
        </div>
        <div className="map-layout">
          <span className="muted small">{t.map.layout}</span>
          <Segmented value={layout} options={[{ value: "down", label: t.map.down }, { value: "across", label: t.map.across }]} onChange={setLayout} />
        </div>
        <Button kind="secondary" small onClick={() => open({ kind: "new_agent" })}>
          <Icon name="plus" /> {t.rail.newAgent}
        </Button>
      </header>
      {agentsState.state === "forbidden" ? <div className="banner banner-bad">{t.common.forbidden}</div> : null}
      {agentsState.state === "error" ? (
        <div className="banner banner-warn"><span className="banner-text">{t.map.listFailed} {agentsState.message}</span></div>
      ) : null}
      {tree.length === 0 ? (
        <Empty title={agentsState.state === "waiting" ? t.map.listWaiting : t.map.emptyTitle} body={agentsState.state === "waiting" ? undefined : t.map.emptyBody}>
          <Button kind="primary" onClick={() => open({ kind: "new_agent" })}>
            <Icon name="plus" /> {t.rail.newAgent}
          </Button>
        </Empty>
      ) : (
        <AgentMap tree={tree} layout={layout} onOpen={(a) => open({ kind: "agent", name: a.key })} />
      )}
    </div>
  );
}
