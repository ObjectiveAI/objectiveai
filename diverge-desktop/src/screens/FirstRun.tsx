import { useState } from "react";
import type { FirstRunView } from "../bindings/FirstRunView";
import { Button, Field, Section } from "../components/ui";
import { api, errorText } from "../lib/ipc";
import { t } from "../strings";

/**
 * The first-run page: what people should call you, the 18-or-older line,
 * and the terms slot. A page in the window, never a popup. Until it's
 * finished there's no you here, and nothing is signed or sent.
 */
export function FirstRun(props: { state: FirstRunView; onDone: (next: FirstRunView) => void }) {
  const [name, setName] = useState("");
  const [adult, setAdult] = useState(false);
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);
  const ready = name.trim() !== "" && adult && !busy;

  const finish = async () => {
    setBusy(true);
    setProblem(null);
    try {
      props.onDone(await api.firstRunFinish(name.trim(), adult));
    } catch (e) {
      setProblem(errorText(e));
      setBusy(false);
    }
  };

  return (
    <div className="page">
      <div className="page-inner">
        <h1 className="page-title">{t.firstRun.title}</h1>
        <p className="muted">{t.firstRun.nothingYet}</p>
        {props.state.state === "earlier" ? (
          <div className="banner banner-warn">
            {t.firstRun.earlier} <strong>{props.state.name}</strong>. {t.firstRun.earlierKeeps}
          </div>
        ) : null}
        <Section title={t.firstRun.nameTitle} note={t.firstRun.nameNote}>
          <Field label={t.firstRun.nameLabel}>
            <input value={name} onChange={(e) => setName(e.target.value)} spellCheck={false} />
          </Field>
        </Section>
        <Section title={t.firstRun.adultTitle}>
          <label className="switch switch-line">
            <input type="checkbox" checked={adult} onChange={(e) => setAdult(e.target.checked)} />
            <span>{t.firstRun.adult}</span>
          </label>
        </Section>
        <Section title={t.firstRun.termsTitle}>
          <p className="muted small">{t.firstRun.termsNotYet}</p>
        </Section>
        <p className="muted small">{t.firstRun.account}</p>
        <div className="row-actions">
          <Button kind="primary" onClick={finish} disabled={!ready}>{busy ? t.firstRun.finishing : t.firstRun.finish}</Button>
          {problem ? <span className="warn small">{problem}</span> : null}
        </div>
      </div>
    </div>
  );
}
