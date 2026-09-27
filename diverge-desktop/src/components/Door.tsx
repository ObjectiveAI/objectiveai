// The door: what a room is, shown once you are let in and before you
// take part. On the wire nothing of a room can be read before the host
// authorizes you, so the door comes right after — and before any verb.

import type { SpaceView } from "../bindings/SpaceView";
import { providerName } from "../lib/format";
import { t } from "../strings";
import { Markdown } from "./Markdown";
import { Button, Chip } from "./ui";

export function Door(props: { space: SpaceView; onStay: () => void; onLeave: () => void }) {
  const s = props.space.summary;
  return (
    <div className="door">
      <div className="door-inner">
        <h1>{t.door.title}</h1>
        <p className="muted">{t.door.youAreIn}</p>
        <div className="door-room">
          <h2>{s.title}</h2>
          <Chip>{t.spaces.kinds[s.kind] ?? s.kind}</Chip>
          <span className="muted small">{t.door.runsOn} <span className="mono">{providerName(s.host)}</span></span>
        </div>
        <p className="door-safe">{t.door.nothingOnYours}</p>

        <section>
          <h3>{t.door.itCanAsk}</h3>
          <p className="muted small">{t.door.itCanAskNote}</p>
          <ul className="door-verbs">
            {props.space.tools.map((x) => (
              <li key={x.name}><strong>{x.title}</strong> <span className="muted">— {x.description}</span></li>
            ))}
          </ul>
        </section>

        <section>
          <h3>{t.door.youKeep}</h3>
          <ul className="door-keep">
            <li>{t.door.keepPosts}</li>
            <li>{t.door.keepReceipts}</li>
            <li>{t.door.keepAnytime}</li>
          </ul>
        </section>

        <section>
          <h3>{t.door.charter}</h3>
          {props.space.charter.trim() ? <Markdown text={props.space.charter} /> : <p className="muted small">{t.door.noCharter}</p>}
        </section>

        <div className="row-actions door-actions">
          <Button kind="primary" onClick={props.onStay}>{t.door.stay}</Button>
          <Button kind="quiet" onClick={props.onLeave}>{t.door.leave}</Button>
        </div>
      </div>
    </div>
  );
}
