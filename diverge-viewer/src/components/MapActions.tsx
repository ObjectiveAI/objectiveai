import { useCallback, useEffect, useState } from "react";
import type { AgentView } from "../bindings/AgentView";
import type { AttachOutcome } from "../bindings/AttachOutcome";
import type { DeleteOutcome } from "../bindings/DeleteOutcome";
import type { TagOutcome } from "../bindings/TagOutcome";
import type { ToolsListed } from "../bindings/ToolsListed";
import { api, errorText } from "../lib/ipc";
import { t } from "../strings";
import { Button, Chip, Escapes, Icon } from "./ui";

/** What you can do with an agent from its card, on a row that opens under it: its tags, which of your tools
 *  it has, and removing it while it's idle. Each is an action in the app's registry; nothing here asks in a
 *  box of its own. What changes shows on the card when the live list says so. */
export function MapActions({ agent }: { agent: AgentView }) {
  const [more, setMore] = useState(false);
  return (
    <>
      <div className="map-actions">
        <Button small kind="tertiary" expanded={more} onClick={() => setMore((m) => !m)}>{more ? t.map.less : t.map.more}</Button>
      </div>
      {more ? (
        <Escapes className="map-more" onEscape={() => setMore(false)}>
          <More agent={agent} />
        </Escapes>
      ) : null}
    </>
  );
}

/** The words for an answer that isn't the one asked for. */
function refused(out: TagOutcome | AttachOutcome | DeleteOutcome): string | null {
  switch (out.outcome) {
    case "done":
    case "deleted":
      return null;
    case "not_found":
    case "no_agent":
      return t.mounts.notFound;
    case "no_tool":
      return t.map.noSuchTool;
    case "active":
      return t.convo.removeActive;
    case "forbidden":
      return t.common.forbidden;
    case "error":
      return out.message;
  }
}

export function More({ agent }: { agent: AgentView }) {
  const [tools, setTools] = useState<ToolsListed | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [tag, setTag] = useState("");
  const [pick, setPick] = useState("");
  const [removing, setRemoving] = useState(false);
  const [busy, setBusy] = useState(false);

  // Your tools, read again whenever this agent's change: which are attached here, and how.
  const toolsNow = agent.tools.join("\n");
  useEffect(() => {
    let gone = false;
    api.toolsList().then(
      (l) => {
        if (gone) return;
        setTools(l);
        if (l.forbidden) setProblem(t.common.forbidden);
        else if (l.errors.length) setProblem(l.errors[0]);
      },
      (e) => !gone && setProblem(errorText(e)),
    );
    return () => {
      gone = true;
    };
  }, [agent.key, toolsNow]);

  const act = useCallback(async (run: () => Promise<TagOutcome | AttachOutcome | DeleteOutcome>) => {
    setBusy(true);
    setProblem(null);
    try {
      setProblem(refused(await run()));
    } catch (e) {
      setProblem(errorText(e));
    } finally {
      setBusy(false);
    }
  }, []);

  const addTag = () => {
    const words = tag.trim();
    if (!words) return;
    setTag("");
    act(() => api.agentsTag(agent.key, [words]));
  };
  const mine = tools?.tools ?? [];
  const direct = (key: string) => mine.some((x) => x.key === key && x.agents.includes(agent.key));
  const free = mine.filter((x) => !agent.tools.includes(x.key));
  const chosen = free.some((x) => x.key === pick) ? pick : (free[0]?.key ?? "");

  return (
    <>
      <div className="map-more-part">
        <span className="map-more-title">{t.map.tags}</span>
        <div className="map-more-row">
          {agent.tags.length === 0 ? <span className="muted small">{t.map.noTags}</span> : null}
          {agent.tags.map((x) => (
            <span key={x} className="map-tag">
              <Chip>{t.map.tagMark}{x}</Chip>
              <Button kind="icon" label={`${t.map.untag} ${x}`} disabled={busy} onClick={() => act(() => api.agentsUntag(agent.key, [x]))}>
                <Icon name="close" />
              </Button>
            </span>
          ))}
        </div>
        <div className="map-more-row">
          <input value={tag} aria-label={t.map.tagLabel} placeholder={t.map.tagPlaceholder} onChange={(e) => setTag(e.target.value)} onKeyDown={(e) => { if (e.key === "Enter") addTag(); }} />
          <Button small kind="secondary" disabled={busy || !tag.trim()} onClick={addTag}>{t.map.addTag}</Button>
        </div>
        <p className="muted small">{t.map.tagsNote}</p>
      </div>

      <div className="map-more-part">
        <span className="map-more-title">{t.map.tools}</span>
        <div className="map-more-row">
          {agent.tools.length === 0 ? <span className="muted small">{t.map.noTools}</span> : null}
          {agent.tools.map((x) => (
            <span key={x} className="map-tag">
              <Chip>{x}</Chip>
              {direct(x) ? (
                <Button small kind="tertiary" disabled={busy} onClick={() => act(() => api.toolsDetach(agent.key, x))}>{t.map.detach}</Button>
              ) : tools ? (
                <span className="muted small">{t.map.withTemplate}</span>
              ) : null}
            </span>
          ))}
        </div>
        {tools && free.length ? (
          <div className="map-more-row">
            <select value={chosen} aria-label={t.map.toolLabel} onChange={(e) => setPick(e.target.value)}>
              {free.map((x) => <option key={x.key} value={x.key}>{x.key}</option>)}
            </select>
            <Button small kind="secondary" disabled={busy || !chosen} onClick={() => act(() => api.toolsAttach(agent.key, chosen))}>{t.map.attach}</Button>
          </div>
        ) : tools ? (
          <p className="muted small">{t.map.noToolsToAttach}</p>
        ) : null}
        <p className="muted small">{t.map.toolsNote}</p>
      </div>

      <div className="map-more-part">
        {agent.active ? (
          <p className="muted small">{t.map.removeWhileWorking}</p>
        ) : removing ? (
          <div className="map-more-row">
            <Button small kind="danger" disabled={busy} onClick={() => act(() => api.agentsDelete(agent.key))}>{t.convo.removeConfirm}</Button>
            <Button small kind="tertiary" onClick={() => setRemoving(false)}>{t.convo.removeCancel}</Button>
          </div>
        ) : (
          <div className="map-more-row">
            <Button small kind="tertiary" onClick={() => setRemoving(true)}>{t.convo.remove}</Button>
          </div>
        )}
      </div>
      {problem ? <p className="bad small" role="status">{problem}</p> : null}
    </>
  );
}
