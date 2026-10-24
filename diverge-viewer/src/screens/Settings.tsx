import { useEffect, useState } from "react";
import { Button, Card, Chip, Row, Section } from "../components/ui";
import { app } from "../lib/app";
import { useShared } from "../lib/context";
import { useAccounts } from "../lib/daemon";
import { t } from "../strings";

export function Settings() {
  const { connected, link, redial, info } = useShared();
  return (
    <div className="page">
      <div className="page-inner">
        <h1 className="page-title">{t.settings.title}</h1>
        <Section title={t.settings.daemon} note={t.settings.daemonNote}>
          <p>
            {link.state === "connected" ? (
              t.daemon.connected
            ) : link.state === "failed" ? (
              <>
                {t.daemon.failed} <span className="mono selectable">{link.message}</span>
              </>
            ) : (
              <>
                <span className="working-dots" aria-hidden /> {t.daemon.dialling}
              </>
            )}
          </p>
          {link.state === "connected" || link.state === "failed" ? (
            <Button kind="secondary" small onClick={redial}>
              {t.daemon.redial}
            </Button>
          ) : null}
        </Section>
        <Accounts connected={connected} />
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

/** The accounts this connection may see, as the daemon lists them: the first thing the whole path proves. */
function Accounts(props: { connected: boolean }) {
  const held = useAccounts(props.connected);
  if (!props.connected) return null;
  return (
    <Section title={t.settings.accounts} note={t.settings.accountsNote}>
      {held.ended ? <p className="muted">{held.ended}</p> : null}
      {held.listed && held.items.length === 0 ? <p className="muted">{t.accounts.none}</p> : null}
      {held.items.map((a) => (
        <Card key={`${a.identity.issuer}\u0000${a.identity.subject}`} tone="quiet">
          <p>
            <strong className="mono selectable">{a.identity.subject}</strong> {t.accounts.at}{" "}
            <span className="mono selectable">{a.identity.issuer}</span>{" "}
            {a.connected ? <Chip tone="ok">{t.accounts.connected}</Chip> : null}
          </p>
          {a.description ? <p className="muted">{a.description}</p> : null}
          {a.roles && a.roles.length > 0 ? (
            <p className="muted">
              {t.accounts.roles}: {a.roles.join(", ")}
            </p>
          ) : null}
        </Card>
      ))}
    </Section>
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
