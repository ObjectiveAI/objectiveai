import { daemon } from "@diverge/sdk";
import { useState } from "react";
import { Button, Card, Field, Row, Section } from "../components/ui";
import { errorText } from "../lib/app";
import { useShared } from "../lib/context";
import { type ListSpec, handle, useListing } from "../lib/daemon";
import { bytes, time } from "../lib/format";
import { t } from "../strings";
import { NotConnected } from "./NotConnected";

const templates = daemon.endpoints.agents.templates;
type Listed = daemon.endpoints.agents.templates.list.server.response.Listed;
const TEMPLATES: ListSpec<Listed, daemon.endpoints.agents.templates.list.server.response.Frame> = {
  open: (h) => templates.list.client.execute(h, {}),
  key: templates.list.server.response.key,
  change: templates.list.server.response.change,
};

const GB = 1024 ** 3;

export function NewAgent() {
  const { connected } = useShared();
  const held = useListing(TEMPLATES, connected);
  const [picked, setPicked] = useState<string | null>(null);
  const [making, setMaking] = useState(false);
  if (!connected) return <NotConnected title={t.create.title} />;
  const template = held.items.find((l) => l.id === picked) ?? null;
  return (
    <div className="page">
      <div className="page-inner">
        <h1 className="page-title">{t.create.title}</h1>
        <Section step={1} title={t.create.template} note={t.create.templateNote} aside={<Button kind="secondary" small onClick={() => setMaking((m) => !m)}>{making ? t.common.cancel : t.create.newTemplate}</Button>}>
          {held.ended ? <p className="muted">{held.ended}</p> : null}
          {held.listed && held.items.length === 0 && !making ? <p className="muted">{t.create.noTemplates}</p> : null}
          {held.items.map((l) => (
            <Row key={l.id} choice on={picked === l.id} onClick={() => setPicked(l.id)} className="template-row">
              <span className="mono">{l.id.slice(0, 12)}</span>
              <span className="template-words">{l.template.description ?? l.template.image.digest}</span>
              <span className="muted">
                {bytes(l.template.memory)} · {bytes(l.template.disk)} · {time(l.created)}
              </span>
            </Row>
          ))}
          {making ? (
            <NewTemplate
              onMade={(id) => {
                setPicked(id);
                setMaking(false);
              }}
            />
          ) : null}
        </Section>
        {template ? <Make template={template} /> : null}
      </div>
    </div>
  );
}

/** A template the daemon has not yet seen: the image, the ceilings, the arguments, the secrets it needs. */
function NewTemplate({ onMade }: { onMade: (id: string) => void }) {
  const [description, setDescription] = useState("");
  const [digest, setDigest] = useState("");
  const [registry, setRegistry] = useState("");
  const [name, setName] = useState("");
  const [memory, setMemory] = useState("2");
  const [disk, setDisk] = useState("4");
  const [args, setArgs] = useState("{}");
  const [secrets, setSecrets] = useState("");
  const [said, setSaid] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const create = async () => {
    let parsed: unknown;
    try {
      parsed = JSON.parse(args);
    } catch {
      setSaid(t.create.badJson);
      return;
    }
    const needs = secrets.split(/[,\s]+/).filter(Boolean);
    const frame: daemon.endpoints.agents.templates.create.client.request.Frame = {
      type: "agent",
      ...(description.trim() ? { description: description.trim() } : {}),
      image: { digest: digest.trim(), ...(registry.trim() && name.trim() ? { references: [{ registry: registry.trim(), name: name.trim() }] } : {}) },
      memory: Math.round(Number(memory) * GB),
      disk: Math.round(Number(disk) * GB),
      arguments: parsed as daemon.endpoints.agents.templates.create.client.request.Frame["arguments"],
      ...(needs.length ? { secrets: needs } : {}),
    };
    setBusy(true);
    setSaid(null);
    try {
      const answer = await templates.create.client.execute(handle, frame);
      if (typeof answer === "object" && "created" in answer) onMade(answer.created);
      else if (typeof answer === "object" && "exists" in answer) onMade(answer.exists);
      else setSaid(answer === "forbidden" ? t.daemon.forbidden : JSON.stringify(answer));
    } catch (error) {
      setSaid(errorText(error));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="fields">
      <Field label={t.create.description}>
        <input value={description} onChange={(e) => setDescription(e.target.value)} />
      </Field>
      <Field label={t.create.digest} hint={t.create.digestNote} needed>
        <input className="mono" value={digest} onChange={(e) => setDigest(e.target.value)} placeholder="sha256:…" />
      </Field>
      <div className="toolbar">
        <Field label={t.create.registry}>
          <input value={registry} onChange={(e) => setRegistry(e.target.value)} placeholder="docker.io" />
        </Field>
        <Field label={t.create.imageName}>
          <input value={name} onChange={(e) => setName(e.target.value)} placeholder="library/python:3.12" />
        </Field>
      </div>
      <div className="toolbar">
        <Field label={t.create.memory} hint={t.create.memoryNote}>
          <input type="number" min={0} step="0.5" value={memory} onChange={(e) => setMemory(e.target.value)} />
        </Field>
        <Field label={t.create.disk} hint={t.create.diskNote}>
          <input type="number" min={0} step="0.5" value={disk} onChange={(e) => setDisk(e.target.value)} />
        </Field>
      </div>
      <Field label={t.create.arguments} hint={t.create.argumentsNote}>
        <textarea className="mono" rows={4} value={args} onChange={(e) => setArgs(e.target.value)} />
      </Field>
      <Field label={t.create.secrets} hint={t.create.secretsNote}>
        <input value={secrets} onChange={(e) => setSecrets(e.target.value)} placeholder="OPENAI_API_KEY, …" />
      </Field>
      {said ? <Card tone="bad">{said}</Card> : null}
      <div className="toolbar">
        <Button kind="primary" onClick={() => void create()} disabled={busy || !digest.trim()}>
          {t.create.makeTemplate}
        </Button>
      </div>
    </div>
  );
}

/** An agent from the picked template: a name, and the secrets the template declares, by name. */
function Make({ template }: { template: Listed }) {
  const [name, setName] = useState("");
  const [secrets, setSecrets] = useState<Record<string, string>>({});
  const [said, setSaid] = useState<string | null>(null);
  const [made, setMade] = useState(false);
  const [busy, setBusy] = useState(false);
  const needs = template.template.secrets ?? [];
  const create = async () => {
    setBusy(true);
    setSaid(null);
    setMade(false);
    try {
      const answer = await daemon.endpoints.agents.create.client.execute(handle, {
        template: template.id,
        ...(name.trim() ? { name: name.trim() } : {}),
        ...(needs.length ? { secrets: Object.fromEntries(needs.map((k) => [k, secrets[k] ?? ""])) } : {}),
      });
      if (answer === "created") {
        setMade(true);
        setName("");
      } else setSaid(answer === "in_use" ? t.create.inUse : answer === "forbidden" ? t.daemon.forbidden : JSON.stringify(answer));
    } catch (error) {
      setSaid(errorText(error));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Section step={2} title={t.create.settings}>
      <div className="fields">
        <Field label={t.create.name} hint={t.create.nameNote}>
          <input value={name} onChange={(e) => setName(e.target.value)} placeholder={t.create.namePlaceholder} />
        </Field>
        {needs.map((key) => (
          <Field key={key} label={key} hint={t.create.secretNote} needed>
            <input type="password" className="mono" value={secrets[key] ?? ""} onChange={(e) => setSecrets((s) => ({ ...s, [key]: e.target.value }))} />
          </Field>
        ))}
      </div>
      {said ? <Card tone="bad">{said}</Card> : null}
      {made ? <Card tone="quiet">{t.create.made}</Card> : null}
      <div className="toolbar">
        <Button kind="primary" onClick={() => void create()} disabled={busy || needs.some((k) => !(secrets[k] ?? "").length)}>
          {busy ? t.create.creating : t.create.create}
        </Button>
      </div>
    </Section>
  );
}
