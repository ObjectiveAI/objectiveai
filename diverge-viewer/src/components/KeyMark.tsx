import { t } from "../strings";

/** A key's mark beside a name: shown only when someone else in the same room goes by that name. */
export function KeyMark(props: { mark: string | null | undefined }) {
  return props.mark ? <span className="mono muted small" title={t.marks.title}>{props.mark}</span> : null;
}
