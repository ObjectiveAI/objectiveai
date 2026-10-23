import { daemon } from "@diverge/sdk";
import { useEffect, useState } from "react";
import { Button, Card, Chip, Escapes, Field, Section, Segmented } from "../components/ui";
import { app, errorText } from "../lib/app";
import { useShared } from "../lib/context";
import { type ListSpec, handle, useListing } from "../lib/daemon";
import { ago, bytes } from "../lib/format";
import { t } from "../strings";
import { NotConnected } from "./NotConnected";

const providers = daemon.endpoints.providers;
type Outgoing = daemon.endpoints.providers.outgoing.list.server.response.Outgoing;
type Incoming = daemon.endpoints.providers.incoming.list.server.response.Incoming;
type Status = daemon.endpoints.providers.Status;
type Capacity = Outgoing["capacity"];

const OUTGOING: ListSpec<Outgoing, daemon.endpoints.providers.outgoing.list.server.response.Frame> = {
  open: (h) => providers.outgoing.list.client.execute(h, {}),
  key: providers.outgoing.list.server.response.key,
  change: providers.outgoing.list.server.response.change,
};
const INCOMING: ListSpec<Incoming, daemon.endpoints.providers.incoming.list.server.response.Frame> = {
  open: (h) => providers.incoming.list.client.execute(h, {}),
  key: providers.incoming.list.server.response.key,
  change: providers.incoming.list.server.response.change,
};

export function Machines() {
  const { connected } = useShared();
  const outgoing = useListing(OUTGOING, connected);
  const incoming = useListing(INCOMING, connected);
  const [names, setNames] = useState<Record<string, string>>({});
  useEffect(() => {
    app.machineNames().then(setNames).catch(() => {});
  }, []);
  const rename = (key: string, name: string) => app.machineRename(key, name).then(setNames).catch(() => {});
  if (!connected) return <NotConnected title={t.machines.title} />;
  const none = outgoing.listed && incoming.listed && outgoing.items.length + incoming.items.length === 0;
  return (
    <div className="page">
      <div className="page-inner">
        <h1 className="page-title">{t.machines.title}</h1>
        <p className="muted">{t.machines.note}</p>
        {outgoing.ended ? <Card tone="bad">{outgoing.ended}</Card> : null}
        {incoming.ended ? <Card tone="bad">{incoming.ended}</Card> : null}
        {none ? <p className="muted">{t.machines.none}</p> : null}
        {outgoing.items.map((m) => (
          <Machine
            key={`out:${m.address}`}
            machineKey={m.address}
            name={names[m.address]}
            kind={t.provider.outgoing}
            status={m.status}
            capacity={m.capacity}
            lastConnected={m.last_connected}
            onRename={(n) => rename(m.address, n)}
            remove={() => providers.outgoing.delete.client.execute(handle, { address: m.address })}
          />
        ))}
        {incoming.items.map((m) => (
          <Machine
            key={`in:${m.identity}`}
            machineKey={m.identity}
            name={names[m.identity]}
            kind={t.provider.incoming}
            status={m.status}
            capacity={m.capacity}
            onRename={(n) => rename(m.identity, n)}
            remove={() => providers.incoming.delete.client.execute(handle, { identity: m.identity })}
          />
        ))}
        <Add />
      </div>
    </div>
  );
}

/** One provider: the name you gave it (editable), what the daemon knows it as, how it stands, its room. */
function Machine(props: { machineKey: string; name: string | undefined; kind: string; status: Status; capacity: Capacity; lastConnected?: string; onRename: (name: string) => void; remove: () => Promise<DeleteAnswer> }) {
  const [editing, setEditing] = useState<string | null>(null);
  const [asking, setAsking] = useState(false);
  const [said, setSaid] = useState<string | null>(null);
  const remove = async () => {
    setAsking(false);
    try {
      const answer = await props.remove();
      if (answer !== "deleted") setSaid(answer === "in_use" ? t.machines.inUse : answer === "not_found" ? t.machines.gone : answer === "forbidden" ? t.daemon.forbidden : JSON.stringify(answer));
    } catch (error) {
      setSaid(errorText(error));
    }
  };
  return (
    <Section
      title={props.name ?? props.machineKey}
      note={`${props.kind} · ${t.machines.daemonName} ${props.machineKey}`}
      aside={
        <span className="toolbar">
          <StatusChip status={props.status} />
          <Button kind="tertiary" small onClick={() => setEditing(editing === null ? (props.name ?? "") : null)}>
            {t.machines.rename}
          </Button>
        </span>
      }
    >
      {editing !== null ? (
        <Escapes onEscape={() => setEditing(null)} className="toolbar">
          <input value={editing} onChange={(e) => setEditing(e.target.value)} placeholder={t.machines.namePlaceholder} />
          <Button
            kind="primary"
            small
            onClick={() => {
              props.onRename(editing);
              setEditing(null);
            }}
          >
            {t.common.ok}
          </Button>
        </Escapes>
      ) : null}
      {props.capacity ? (
        <p className="muted">
          {t.machines.room}: {t.machines.volumes} {bytes(props.capacity.volumes_disk)} · {t.machines.images} {bytes(props.capacity.images_disk)} · {t.machines.containerMemory} {bytes(props.capacity.container_memory)} · {t.machines.containerDisk} {bytes(props.capacity.container_disk)}
        </p>
      ) : null}
      {props.status.type !== "disconnected" ? (
        <p className="muted">
          {t.machines.version} {props.status.info.version}
        </p>
      ) : props.lastConnected ? (
        <p className="muted">
          {t.machines.lastConnected} {ago(props.lastConnected)}
        </p>
      ) : null}
      {said ? <Card tone="bad">{said}</Card> : null}
      {asking ? (
        <Escapes onEscape={() => setAsking(false)} className="toolbar">
          <Button kind="danger" small onClick={() => void remove()}>
            {t.machines.confirm}
          </Button>
          <Button kind="tertiary" small onClick={() => setAsking(false)}>
            {t.machines.keep}
          </Button>
        </Escapes>
      ) : (
        <Button kind="tertiary" small onClick={() => setAsking(true)}>
          {t.machines.remove}
        </Button>
      )}
    </Section>
  );
}

type DeleteAnswer = daemon.endpoints.providers.outgoing.delete.server.response.Frame;

function StatusChip({ status }: { status: Status }) {
  switch (status.type) {
    case "connected":
      return <Chip tone="ok">{t.machines.connected}</Chip>;
    case "different_version":
      return <Chip tone="warn">{t.machines.differentVersion}</Chip>;
    case "disconnected":
      return <Chip tone="plain">{t.machines.disconnected}</Chip>;
  }
}

/** Add a provider: one the daemon dials, presenting the key a keyring entry holds; or one that will dial the daemon, whose new key the daemon keeps in its keyring. */
function Add() {
  const [way, setWay] = useState<"dial" | "accept">("dial");
  const [address, setAddress] = useState("");
  const [key, setKey] = useState("");
  const [identity, setIdentity] = useState("");
  const [said, setSaid] = useState<string | null>(null);
  const [minted, setMinted] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const add = async () => {
    setBusy(true);
    setSaid(null);
    setMinted(null);
    try {
      if (way === "dial") {
        const answer = await providers.outgoing.add.client.execute(handle, { address: address.trim(), mode: { unbrokered: { key: key.trim() } } });
        if (answer === "added") {
          setAddress("");
          setKey("");
        } else setSaid(answer === "exists" ? t.machines.exists : answer === "forbidden" ? t.daemon.forbidden : JSON.stringify(answer));
      } else {
        const answer = await providers.incoming.add.client.execute(handle, { identity: identity.trim() });
        if (typeof answer === "object" && "added" in answer) {
          setMinted(answer.added);
          setIdentity("");
        } else setSaid(answer === "exists" ? t.machines.exists : answer === "forbidden" ? t.daemon.forbidden : JSON.stringify(answer));
      }
    } catch (error) {
      setSaid(errorText(error));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Section title={t.machines.add}>
      <Segmented
        value={way}
        options={[
          { value: "dial", label: t.machines.dial },
          { value: "accept", label: t.machines.accept },
        ]}
        onChange={setWay}
      />
      {way === "dial" ? (
        <div className="fields">
          <Field label={t.machines.address} needed>
            <input value={address} onChange={(e) => setAddress(e.target.value)} placeholder={t.machines.addressPlaceholder} />
          </Field>
          <Field label={t.machines.key} hint={t.machines.keyNote} needed>
            <input className="mono" value={key} onChange={(e) => setKey(e.target.value)} placeholder={t.machines.keyPlaceholder} />
          </Field>
        </div>
      ) : (
        <div className="fields">
          <Field label={t.machines.identity} hint={t.machines.identityNote} needed>
            <input value={identity} onChange={(e) => setIdentity(e.target.value)} placeholder={t.machines.identityPlaceholder} />
          </Field>
        </div>
      )}
      {said ? <Card tone="bad">{said}</Card> : null}
      {minted ? (
        <Card tone="quiet">
          <strong>{t.machines.mintedTitle}</strong>
          <span className="muted">{t.machines.mintedNote}</span>
          <span className="mono selectable key-once">{minted}</span>
        </Card>
      ) : null}
      <Button kind="primary" onClick={() => void add()} disabled={busy || (way === "dial" ? !address.trim() || !key.trim() : !identity.trim())}>
        {t.machines.save}
      </Button>
    </Section>
  );
}
