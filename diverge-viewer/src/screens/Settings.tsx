import { useEffect, useState } from "react";
import { Row, Section } from "../components/ui";
import { app } from "../lib/app";
import { useShared } from "../lib/context";
import { t } from "../strings";

export function Settings() {
  const { connected, info } = useShared();
  return (
    <div className="page">
      <div className="page-inner">
        <h1 className="page-title">{t.settings.title}</h1>
        <Section title={t.settings.daemon} note={t.settings.daemonNote}>
          <p>
            {connected ? null : <span className="working-dots" aria-hidden />} {connected ? t.daemon.connected : t.daemon.notConnected}
          </p>
        </Section>
        <ThemePicker />
        {info ? (
          <Section title={t.settings.folder} note={t.settings.folderNote}>
            <p className="mono selectable">{info.folder}</p>
          </Section>
        ) : null}
      </div>
    </div>
  );
}

/** The themes in theme.css, each shown as its five colours and its name. A click puts it on at once. */
function ThemePicker() {
  const [on, setOn] = useState<string | null>(null);
  useEffect(() => {
    app.theme().then((id) => setOn(id ?? Object.keys(t.settings.themes)[0])).catch(() => {});
  }, []);
  const pick = (id: string) =>
    app.themeSet(id).then((kept) => {
      if (kept) document.documentElement.dataset.theme = kept;
      setOn(kept);
    });
  return (
    <Section title={t.settings.theme} note={t.settings.themeNote}>
      <div className="themes">
        {Object.entries(t.settings.themes).map(([id, name]) => (
          <Row key={id} className="theme" choice on={on === id} onClick={() => pick(id)}>
            <span className="theme-swatch" data-theme={id} aria-hidden>
              <span className="sw-ground" />
              <span className="sw-raised" />
              <span className="sw-accent" />
              <span className="sw-ink" />
              <span className="sw-here" />
            </span>
            <span className="theme-name">{name}</span>
          </Row>
        ))}
      </div>
    </Section>
  );
}
