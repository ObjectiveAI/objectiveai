import type { MouseEvent, ReactNode } from "react";

/** How a button looks: the one action that goes ahead (primary), an ordinary one (secondary), a quiet one
 *  (tertiary), one that can't be undone (danger), or an icon alone, which must carry a label. */
export type ButtonKind = "primary" | "secondary" | "tertiary" | "danger" | "icon";

type ButtonProps = {
  onClick?: () => void;
  onMouseDown?: (e: MouseEvent<HTMLButtonElement>) => void;
  disabled?: boolean;
  small?: boolean;
  title?: string;
  type?: "button" | "submit";
} & ({ kind: Exclude<ButtonKind, "icon">; children: ReactNode } | { kind: "icon"; label: string; children?: ReactNode });

export function Button(props: ButtonProps) {
  const label = props.kind === "icon" ? props.label : undefined;
  return (
    <button
      type={props.type ?? "button"}
      className={`btn btn-${props.kind}${props.small ? " btn-small" : ""}`}
      onClick={props.onClick}
      onMouseDown={props.onMouseDown}
      disabled={props.disabled}
      title={props.title ?? label}
      aria-label={label}
    >
      {props.children}
    </button>
  );
}

export function Segmented<T extends string>(props: { value: T; options: { value: T; label: string }[]; onChange: (v: T) => void }) {
  return (
    <div className="segmented" role="radiogroup">
      {props.options.map((o) => (
        <button type="button" key={o.value} role="radio" aria-checked={props.value === o.value} className={props.value === o.value ? "on" : ""} onClick={() => props.onChange(o.value)}>
          {o.label}
        </button>
      ))}
    </div>
  );
}

/** A row of views over one place (Home's filters): each a tab, the one you're on marked. */
export function Tabs<T extends string>(props: { value: T; options: { value: T; label: string }[]; onChange: (v: T) => void; className?: string }) {
  return (
    <div className={`view-tabs${props.className ? ` ${props.className}` : ""}`} role="tablist">
      {props.options.map((o) => (
        <button type="button" key={o.value} role="tab" aria-selected={props.value === o.value} className={`view-tab${props.value === o.value ? " on" : ""}`} onClick={() => props.onChange(o.value)}>
          {o.label}
        </button>
      ))}
    </div>
  );
}

export function Field(props: { label: string; hint?: string; children: ReactNode; needed?: boolean; wide?: boolean; size?: "narrow" | "grow" }) {
  return (
    <label className={`field${props.wide ? " field-wide" : ""}${props.size ? ` field-${props.size}` : ""}`}>
      <span className="field-label">
        {props.label}
        {props.needed ? <span className="needed">needed</span> : null}
      </span>
      {props.children}
      {props.hint ? <span className="field-hint">{props.hint}</span> : null}
    </label>
  );
}

/** The head of a section: its title (and step number, note, and anything beside it). The small form
 *  heads a list inside a column or a section. */
export function SectionHead(props: { title: ReactNode; note?: string; step?: number; aside?: ReactNode; level?: 2 | 3; small?: boolean }) {
  const H = props.level === 3 ? "h3" : "h2";
  return (
    <header className={`section-head${props.small ? " section-head-small" : ""}`}>
      {props.step !== undefined ? <span className="step">{props.step}</span> : null}
      <div className="section-titles">
        <H>{props.title}</H>
        {props.note ? <p className="muted">{props.note}</p> : null}
      </div>
      {props.aside}
    </header>
  );
}

export function Section(props: { step?: number; title: string; note?: string; children: ReactNode; aside?: ReactNode }) {
  return (
    <section className="section">
      <SectionHead step={props.step} title={props.title} note={props.note} aside={props.aside} />
      <div className="section-body">{props.children}</div>
    </section>
  );
}

export function Dot(props: { state: "working" | "idle" | "never" | "bad" }) {
  return <span className={`dot dot-${props.state}`} aria-hidden />;
}

/** A chip: a label (a state, a kind) with a tint and no edge, or, given onClick, a pressable one (a room)
 *  with an edge. */
export function Chip(props: { children: ReactNode; title?: string } & ({ tone?: "ok" | "warn" | "bad" | "plain" | "accent"; onClick?: undefined } | { onClick: () => void; tone?: undefined })) {
  if (props.onClick) {
    return (
      <button type="button" className="chip chip-press" title={props.title} onClick={props.onClick}>
        {props.children}
      </button>
    );
  }
  return (
    <span className={`chip chip-${props.tone ?? "plain"}`} title={props.title}>
      {props.children}
    </span>
  );
}

/** A card: one padding, one rhythm inside. Given onClick, the whole card is a button. */
export function Card(props: { children: ReactNode; tone?: "bad" | "warn" | "quiet"; className?: string; onClick?: () => void }) {
  if (props.onClick) {
    return (
      <button type="button" className={`card card-press${props.tone ? ` card-${props.tone}` : ""}${props.className ? ` ${props.className}` : ""}`} onClick={props.onClick}>
        {props.children}
      </button>
    );
  }
  return <div className={`card${props.tone ? ` card-${props.tone}` : ""}${props.className ? ` ${props.className}` : ""}`}>{props.children}</div>;
}

/** A row you press to go somewhere (the rail, a list, a tree). `on` marks the one where you are; with
 *  `choice`, it marks the one picked among several instead. */
export function Row(props: { children: ReactNode; onClick: () => void; on?: boolean; choice?: boolean; className?: string; title?: string }) {
  return (
    <button
      type="button"
      className={`nav-row${props.className ? ` ${props.className}` : ""}${props.on ? " on" : ""}`}
      onClick={props.onClick}
      title={props.title}
      aria-current={!props.choice && props.on ? "true" : undefined}
      aria-pressed={props.choice ? !!props.on : undefined}
    >
      {props.children}
    </button>
  );
}

export function Empty(props: { title: string; body?: string; children?: ReactNode }) {
  return (
    <div className="empty">
      <h3>{props.title}</h3>
      {props.body ? <p className="muted">{props.body}</p> : null}
      {props.children}
    </div>
  );
}

export function Icon(props: { name: "plus" | "close" | "chevron" | "folder" | "file" | "link" | "agent" | "machine" | "view" | "files" | "home" }) {
  const common = { width: 14, height: 14, viewBox: "0 0 16 16", fill: "none", stroke: "currentColor", strokeWidth: 1.5, strokeLinecap: "round" as const, strokeLinejoin: "round" as const };
  switch (props.name) {
    case "plus":
      return <svg {...common}><path d="M8 3v10M3 8h10" /></svg>;
    case "close":
      return <svg {...common}><path d="M4 4l8 8M12 4l-8 8" /></svg>;
    case "chevron":
      return <svg {...common}><path d="M6 4l4 4-4 4" /></svg>;
    case "folder":
      return <svg {...common}><path d="M2 4.5h4l1.5 1.5H14v6.5H2z" /></svg>;
    case "file":
      return <svg {...common}><path d="M4 2h5l3 3v9H4z M9 2v3h3" /></svg>;
    case "link":
      return <svg {...common}><path d="M7 9l2-2M6 11l-1 1a2 2 0 01-3-3l2-2M10 5l1-1a2 2 0 013 3l-2 2" /></svg>;
    case "agent":
      return <svg {...common}><circle cx="8" cy="6" r="3" /><path d="M3 14c.8-2.5 2.8-4 5-4s4.2 1.5 5 4" /></svg>;
    case "machine":
      return <svg {...common}><rect x="2" y="3" width="12" height="8" rx="1.5" /><path d="M6 14h4M8 11v3" /></svg>;
    case "view":
      return <svg {...common}><path d="M2 4h12M4 8h8M6 12h4" /></svg>;
    case "files":
      return <svg {...common}><path d="M3 3h4l1 1h5v8H3z" /><path d="M5 12v1.5h9V6" /></svg>;
    case "home":
      return <svg {...common}><path d="M2.5 8L8 3l5.5 5" /><path d="M4 7v6h8V7" /></svg>;
  }
}
