import type { CardLink } from "../bindings/CardLink";
import { t } from "../strings";

/** A card as someone sees it: a picture, a few words, links as text (the app opens no web pages). */
export function ProfileCard(props: { name: string; picture?: string | null; about?: string | null; links?: CardLink[] }) {
  const links = props.links ?? [];
  const empty = !props.picture && !props.about && links.length === 0;
  return (
    <div className="profile-card">
      {props.picture ? <img className="profile-card-picture" src={props.picture} alt="" /> : <span className="profile-card-picture profile-card-initial" aria-hidden>{props.name.slice(0, 1).toUpperCase()}</span>}
      <div className="profile-card-text">
        <strong>{props.name}</strong>
        {props.about ? <p className="profile-card-about">{props.about}</p> : null}
        {links.length ? (
          <ul className="profile-card-links">
            {links.map((l, i) => (
              <li key={i}>
                {l.title} <span className="mono muted small selectable">{l.url}</span>
              </li>
            ))}
          </ul>
        ) : null}
        {empty ? <p className="muted small">{t.profile.card.nothingSeen}</p> : null}
      </div>
    </div>
  );
}
