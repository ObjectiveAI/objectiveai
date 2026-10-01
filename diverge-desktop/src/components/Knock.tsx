import type { KnockView } from "../bindings/KnockView";
import { ago } from "../lib/format";
import { t } from "../strings";
import { Button } from "./ui";

/** Someone at the door of a room you host: who they say they are, their note, and who vouches. */
export function KnockCard(props: { knock: KnockView; onAnswer: (yes: boolean) => void; compact?: boolean }) {
  const k = props.knock;
  const vouch = k.vouch;
  return (
    <div className={`knock${props.compact ? " knock-compact" : ""}`}>
      <div className="knock-main">
        <div>
          <strong>{k.name}</strong> <span className="muted small">→ {k.space_title}</span>
        </div>
        {k.note ? <div className="knock-note">“{k.note}”</div> : null}
        <div className="muted small">
          {k.invited ? t.spaces.invitedKnock : t.spaces.openKnock} · {t.spaces.seenFrom} <span className="mono">{k.address}</span> · {ago(k.at)}
          {k.listed ? "" : ` · ${t.spaces.unlisted}`}
        </div>
        {k.checked ? null : <div className="warn small">{t.spaces.knockUnchecked}</div>}
        {vouch ? (
          <div className={vouch.holds && vouch.member_here ? "ok small" : "warn small"}>
            {vouch.holds ? `${t.spaces.vouchedBy} ${vouch.by}${vouch.member_here ? "" : `, ${t.spaces.vouchNotHere}`}` : t.spaces.vouchBroken}
          </div>
        ) : null}
      </div>
      <div className="row-actions">
        <Button small kind="primary" disabled={!k.checked} onClick={() => props.onAnswer(true)}>{t.spaces.letIn}</Button>
        <Button small kind="tertiary" onClick={() => props.onAnswer(false)}>{t.spaces.notNow}</Button>
      </div>
    </div>
  );
}
