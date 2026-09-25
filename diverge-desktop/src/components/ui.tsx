import type { ReactNode } from "react";

export function Button(props: {
  children: ReactNode;
  onClick?: () => void;
  kind?: "primary" | "quiet" | "danger" | "plain";
  disabled?: boolean;
  small?: boolean;
  title?: string;
  type?: "button" | "submit";
}) {
  const { kind = "plain", small, ...rest } = props;
  return (
    <button type={props.type ?? "button"} className={`btn btn-${kind}${small ? " btn-small" : ""}`} onClick={rest.onClick} disabled={rest.disabled} title={rest.title}>
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

export function Field(props: { label: string; hint?: string; children: ReactNode; needed?: boolean; wide?: boolean; size?: "narrow" | "grow" }) {
  return (
    <label className={`field${props.wide ? " field-wide" : ""}${props.size ? ` ${props.size}` : ""}`}>
      <span className="field-label">
        {props.label}
        {props.needed ? <span className="needed">needed</span> : null}
      </span>
      {props.children}
      {props.hint ? <span className="field-hint">{props.hint}</span> : null}
    </label>
  );
}

export function Section(props: { step?: number; title: string; note?: string; children: ReactNode; aside?: ReactNode }) {
  return (
    <section className="section">
      <header className="section-head">
        {props.step !== undefined ? <span className="step">{props.step}</span> : null}
        <div className="section-titles">
          <h2>{props.title}</h2>
          {props.note ? <p className="muted">{props.note}</p> : null}
        </div>
        {props.aside}
      </header>
      <div className="section-body">{props.children}</div>
    </section>
  );
}

export function Dot(props: { state: "working" | "idle" | "never" | "bad" }) {
  return <span className={`dot dot-${props.state}`} aria-hidden />;
}

export function Chip(props: { children: ReactNode; tone?: "ok" | "warn" | "bad" | "plain" | "accent"; title?: string }) {
  return (
    <span className={`chip chip-${props.tone ?? "plain"}`} title={props.title}>
      {props.children}
    </span>
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

export function Icon(props: { name: "plus" | "close" | "chevron" | "folder" | "file" | "link" | "agent" | "machine" | "view" | "files" }) {
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
  }
}
