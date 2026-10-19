import { useEffect, useRef, useState } from "react";
import type { Card } from "../bindings/Card";
import type { ProfileCardView } from "../bindings/ProfileCardView";
import type { Who } from "../bindings/Who";
import { time } from "../lib/format";
import { api, errorText } from "../lib/ipc";
import { t } from "../strings";
import { ProfileCard } from "./ProfileCard";
import { Button, Section, Segmented } from "./ui";

const WHO: Who[] = ["link", "room", "you"];
const LINKS = 5;
/** The most a card's picture may be as a data: address (the room's CARD_PICTURE). */
const PICTURE_MAX = 96_000;

/** A picture made small enough for a card: a square of `side` pixels, as JPEG. */
function shrink(file: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(file);
    const img = new Image();
    img.onload = () => {
      URL.revokeObjectURL(url);
      for (const [side, q] of [[256, 0.85], [192, 0.75], [128, 0.7]] as const) {
        const c = document.createElement("canvas");
        c.width = side;
        c.height = side;
        const s = Math.min(img.width, img.height);
        c.getContext("2d")?.drawImage(img, (img.width - s) / 2, (img.height - s) / 2, s, s, 0, 0, side, side);
        const out = c.toDataURL("image/jpeg", q);
        if (out.length <= PICTURE_MAX) return resolve(out);
      }
      reject(new Error(t.profile.card.tooBig));
    };
    img.onerror = () => {
      URL.revokeObjectURL(url);
      reject(new Error(t.profile.card.notAPicture));
    };
    img.src = url;
  });
}

/** Who sees one part: three choices, and what each means. */
function WhoSees(props: { value: Who; onChange: (w: Who) => void }) {
  return (
    <>
      <Segmented value={props.value} options={WHO.map((w) => ({ value: w, label: t.profile.card.who[w] }))} onChange={props.onChange} />
      <span className="muted small">{t.profile.card.whoNote[props.value]}</span>
    </>
  );
}

/** Your card: each part with who sees it, saved here and sent only where you set it. */
export function CardEditor(props: { name: string }) {
  const [view, setView] = useState<ProfileCardView | null>(null);
  const [card, setCard] = useState<Card | null>(null);
  const [said, setSaid] = useState<{ ok: boolean; text: string } | null>(null);
  const [saving, setSaving] = useState(false);
  const [seenAs, setSeenAs] = useState<"link" | "room">("link");
  const [dropping, setDropping] = useState(false);
  const [invite, setInvite] = useState<string | null>(null);
  const picker = useRef<HTMLInputElement>(null);
  const load = () => api.profileCard().then((v) => { setView(v); setCard(v.card); });
  useEffect(() => { load(); }, []);
  if (!view || !card) return null;

  const set = (patch: Partial<Card>) => { setCard({ ...card, ...patch }); setSaid(null); };
  const take = async (file: Blob | undefined | null) => {
    if (!file) return;
    try { set({ picture: await shrink(file) }); } catch (e) { setSaid({ ok: false, text: errorText(e) }); }
  };
  const save = async () => {
    setSaving(true);
    try {
      const out = await api.profileCardSet(card);
      if (out.outcome === "error") setSaid({ ok: false, text: out.message });
      else {
        const first = !view.profile ? t.profile.card.noProfile : out.posted ? t.profile.card.posted : card.picture_who === "you" && card.about_who === "you" && card.links_who === "you" ? t.profile.card.keptOnly : t.profile.card.unchanged;
        setSaid({ ok: true, text: [first, out.took_back ? t.profile.card.tookBack : ""].filter(Boolean).join(" ") });
        setInvite(null);
        await load();
      }
    } catch (e) {
      setSaid({ ok: false, text: errorText(e) });
    }
    setSaving(false);
  };
  const showLink = async () => {
    if (!view.profile) return;
    const i = await api.spaceInvite(view.profile);
    setInvite(i?.text ?? null);
  };
  /** What someone sees of the card, at the door or let in. */
  const seen = (place: "link" | "room") => {
    const sees = (w: Who) => (place === "link" ? w === "link" : w !== "you");
    const links = card.links.filter((l) => l.title.trim() || l.url.trim());
    return { picture: sees(card.picture_who) ? card.picture || null : null, about: sees(card.about_who) ? card.about.trim() || null : null, links: sees(card.links_who) ? links : [] };
  };

  return (
    <Section title={t.profile.card.title} note={t.profile.card.note}>
      <p className="muted small">{t.profile.card.starting}</p>
      {view.from_room ? <div className="banner banner-quiet">{t.profile.card.fromRoom}</div> : null}

      <div className="card-part">
        <div className="card-part-head"><strong>{t.profile.card.picture}</strong><WhoSees value={card.picture_who} onChange={(w) => set({ picture_who: w })} /></div>
        <div
          className={`card-picture-row${dropping ? " dropping" : ""}`}
          onDragOver={(e) => { e.preventDefault(); setDropping(true); }}
          onDragLeave={() => setDropping(false)}
          onDrop={(e) => { e.preventDefault(); setDropping(false); take(e.dataTransfer.files[0]); }}
        >
          {card.picture ? <img className="profile-card-picture" src={card.picture} alt="" /> : <span className="profile-card-picture profile-card-initial" aria-hidden>{props.name.slice(0, 1).toUpperCase()}</span>}
          <input ref={picker} className="visually-hidden" type="file" accept="image/png,image/jpeg,image/webp" tabIndex={-1} onChange={(e) => { take(e.target.files?.[0]); e.target.value = ""; }} />
          <Button small kind="secondary" onClick={() => picker.current?.click()}>{t.profile.card.choosePicture}</Button>
          <span className="muted small">{t.profile.card.dropPicture}</span>
          {card.picture ? <Button small kind="tertiary" onClick={() => set({ picture: "" })}>{t.profile.card.removePicture}</Button> : null}
        </div>
      </div>

      <div className="card-part">
        <div className="card-part-head"><strong>{t.profile.card.about}</strong><WhoSees value={card.about_who} onChange={(w) => set({ about_who: w })} /></div>
        <textarea rows={3} maxLength={1000} value={card.about} placeholder={t.profile.card.aboutPlaceholder} onChange={(e) => set({ about: e.target.value })} />
      </div>

      <div className="card-part">
        <div className="card-part-head"><strong>{t.profile.card.links}</strong><WhoSees value={card.links_who} onChange={(w) => set({ links_who: w })} /></div>
        {card.links.map((l, i) => (
          <div key={i} className="card-link-row">
            <input value={l.title} maxLength={80} placeholder={t.profile.card.linkTitle} onChange={(e) => set({ links: card.links.map((x, j) => (j === i ? { ...x, title: e.target.value } : x)) })} />
            <input className="mono" value={l.url} placeholder={t.profile.card.linkUrl} spellCheck={false} onChange={(e) => set({ links: card.links.map((x, j) => (j === i ? { ...x, url: e.target.value } : x)) })} />
            <Button small kind="tertiary" onClick={() => set({ links: card.links.filter((_, j) => j !== i) })}>{t.profile.card.removeLink}</Button>
          </div>
        ))}
        {card.links.length < LINKS ? <span><Button small kind="tertiary" onClick={() => set({ links: [...card.links, { title: "", url: "" }] })}>{t.profile.card.addLink}</Button></span> : null}
        <span className="muted small">{t.profile.card.linksAsText}</span>
      </div>

      <div className="card-part">
        <span><Button kind="primary" onClick={save} disabled={saving}>{saving ? t.profile.card.saving : t.profile.card.save}</Button></span>
        {said ? <p className={said.ok ? "ok small" : "warn small"}>{said.text}</p> : null}
        <span className="muted small">
          {!view.profile ? t.profile.card.noProfile : view.in_room ? `${t.profile.card.inRoomSince} ${time(view.in_room)}.` : t.profile.card.noneInRoom}
        </span>
      </div>

      <div className="card-part card-seen">
        <div className="card-part-head">
          <strong>{t.profile.card.seenAs}</strong>
          <Segmented value={seenAs} options={[{ value: "link", label: t.profile.card.seenAsLink }, { value: "room", label: t.profile.card.seenAsRoom }]} onChange={setSeenAs} />
        </div>
        <ProfileCard name={props.name} {...seen(seenAs)} />
        {seenAs === "link" ? <span className="muted small">{t.profile.card.doorAlso}</span> : null}
        {view.profile ? (
          invite ? (
            <div className="banner banner-quiet">
              <span className="muted small">{t.profile.card.linkNote}</span>
              <code className="selectable invite-text">{invite}</code>
              <Button small kind="tertiary" onClick={() => setInvite(null)}>{t.profile.card.hideLink}</Button>
            </div>
          ) : (
            <span><Button small kind="tertiary" onClick={showLink}>{t.profile.card.showLink}</Button></span>
          )
        ) : null}
      </div>
    </Section>
  );
}
