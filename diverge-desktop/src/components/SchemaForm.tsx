// One form system for all six agent images. It renders whatever JSON
// Schema an image publishes (compiled from Ronald's source), so when he
// changes an image's settings the form changes with it. Nothing here is
// specific to one image.

import { useState } from "react";
import { t } from "../strings";
import { Segmented } from "./ui";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type Schema = Record<string, any>;
type Value = unknown;

const SECRET = /(^|_)(key|secret|token|password)$/;
const LONG = /(prompt|python|system|source|bio|message|text)$/;

function resolve(s: Schema, root: Schema): Schema {
  let out = s;
  let guard = 0;
  while (out && typeof out.$ref === "string" && guard++ < 20) {
    const name = out.$ref.replace("#/$defs/", "");
    const { $ref: _ignored, ...rest } = out;
    out = { ...(root.$defs?.[name] ?? {}), ...rest };
  }
  return out ?? {};
}

function constOf(s: Schema): Value | undefined {
  if ("const" in s) return s.const;
  if (Array.isArray(s.enum) && s.enum.length === 1) return s.enum[0];
  return undefined;
}

function variants(s: Schema, root: Schema): Schema[] | null {
  const list = s.oneOf ?? s.anyOf;
  return Array.isArray(list) ? list.map((v: Schema) => resolve(v, root)) : null;
}

/** `T | null` in any of its spellings, as the non-null half. */
function nullable(s: Schema, root: Schema): Schema | null {
  if (Array.isArray(s.type) && s.type.includes("null")) {
    const rest = s.type.filter((x: string) => x !== "null");
    return rest.length === 1 ? { ...s, type: rest[0] } : null;
  }
  const list = variants(s, root);
  if (list && list.length === 2) {
    const other = list.find((v) => v.type !== "null");
    if (other && list.some((v) => v.type === "null")) return { ...other, description: s.description ?? other.description };
  }
  return null;
}

function choices(s: Schema, root: Schema): Value[] | null {
  if (Array.isArray(s.enum) && s.enum.length > 1) return s.enum;
  const list = variants(s, root);
  if (list && list.length > 0 && list.every((v) => constOf(v) !== undefined)) return list.map((v) => constOf(v)!);
  return null;
}

function discriminator(list: Schema[], root: Schema): string | null {
  const first = list[0]?.properties;
  if (!first) return null;
  for (const key of Object.keys(first)) {
    if (list.every((v) => v.properties?.[key] && constOf(resolve(v.properties[key], root)) !== undefined)) return key;
  }
  return null;
}

const ACRONYMS = new Set(["api", "mcp", "url", "id", "ai", "gpu", "cpu", "tts", "llm", "oauth", "cn", "xai"]);

function human(name: string): string {
  const words = name.replace(/_/g, " ").split(" ").map((w) => (ACRONYMS.has(w.toLowerCase()) ? w.toUpperCase() : w));
  const s = words.join(" ");
  return s.charAt(0).toUpperCase() + s.slice(1);
}

function hint(s: Schema): string | undefined {
  const d: string | undefined = s.description;
  if (!d) return undefined;
  const first = d.split(/\n\n/)[0].replace(/\n/g, " ").replace(/`/g, "");
  const sentence = first.match(/^(.+?[.!?])(\s|$)/)?.[1] ?? first;
  return sentence.length > 160 ? sentence.slice(0, 157) + "…" : sentence;
}

function typeOf(s: Schema): string | undefined {
  return Array.isArray(s.type) ? s.type[0] : s.type;
}

function variantLabel(v: Schema, root: Schema, disc: string | null, i: number): string {
  if (disc) return human(String(constOf(resolve(v.properties[disc], root))));
  const ty = typeOf(v);
  if (ty === "object" && Array.isArray(v.required) && v.required.length) return human(v.required[0]);
  if (ty === "array") return "Several";
  if (ty === "string") return "One";
  if (ty === "boolean") return "Yes / no";
  return `Option ${i + 1}`;
}

function matches(v: Schema, value: Value): boolean {
  const ty = typeOf(v);
  if (ty === "array") return Array.isArray(value);
  if (ty === "object") {
    if (!value || typeof value !== "object" || Array.isArray(value)) return false;
    return (v.required ?? []).every((k: string) => k in (value as object));
  }
  if (ty === "string") return typeof value === "string";
  if (ty === "boolean") return typeof value === "boolean";
  if (ty === "integer" || ty === "number") return typeof value === "number";
  return false;
}

/** A starting value: required things filled, optional things absent. */
export function initial(schema: Schema, root: Schema): Value {
  const s = resolve(schema, root);
  if ("default" in s && s.default !== undefined && !(typeof s.default === "object" && s.default && Object.keys(s.default).length === 0)) return s.default;
  const c = constOf(s);
  if (c !== undefined) return c;
  if (nullable(s, root)) return undefined;
  const ch = choices(s, root);
  if (ch) return ch[0];
  const list = variants(s, root);
  if (list) return initial(list[0], root);
  switch (typeOf(s)) {
    case "object": {
      if (!s.properties) return {};
      const out: Record<string, Value> = {};
      for (const key of s.required ?? []) out[key] = initial(s.properties[key], root);
      return out;
    }
    case "array":
      return [];
    case "string":
      return "";
    case "integer":
    case "number":
      return 0;
    case "boolean":
      return false;
    default:
      return {};
  }
}

/** Required text left empty, by field name. The image's own types accept
 *  an empty string, but an agent with no model is no use to anyone. */
export function missing(schema: Schema, value: Value, root: Schema = schema): string[] {
  const s = resolve(schema, root);
  if (constOf(s) !== undefined || nullable(s, root) || choices(s, root)) return [];
  const list = variants(s, root);
  if (list) {
    const disc = discriminator(list, root);
    const chosen =
      (disc && value && typeof value === "object" ? list.find((v) => constOf(resolve(v.properties[disc], root)) === (value as Record<string, Value>)[disc]) : undefined) ??
      list.find((v) => matches(v, value)) ??
      list[0];
    return missing(chosen, value, root);
  }
  if (typeOf(s) === "string") return value ? [] : ["?"];
  if (typeOf(s) === "object" && s.properties) {
    const obj = (value && typeof value === "object" ? value : {}) as Record<string, Value>;
    return (s.required ?? []).flatMap((key: string) => missing(s.properties[key], obj[key], root).map((m) => (m === "?" ? human(key) : m)));
  }
  return [];
}

type FieldProps = { schema: Schema; root: Schema; value: Value; onChange: (v: Value) => void; name: string; required: boolean; depth: number };

export function SchemaForm(props: { schema: Schema; value: Value; onChange: (v: Value) => void }) {
  return <ObjectFields schema={props.schema} root={props.schema} value={props.value} onChange={props.onChange} depth={0} />;
}

function ObjectFields(props: { schema: Schema; root: Schema; value: Value; onChange: (v: Value) => void; depth: number; omit?: string }) {
  const s = resolve(props.schema, props.root);
  const obj = (props.value && typeof props.value === "object" && !Array.isArray(props.value) ? props.value : {}) as Record<string, Value>;
  const required: string[] = s.required ?? [];
  const keys = Object.keys(s.properties ?? {}).filter((k) => k !== props.omit);
  const set = (key: string, v: Value) => {
    const next = { ...obj };
    if (v === undefined) delete next[key];
    else next[key] = v;
    props.onChange(next);
  };
  const field = (key: string) => (
    <SchemaField key={key} name={key} schema={s.properties[key]} root={props.root} value={obj[key]} onChange={(v) => set(key, v)} required={required.includes(key)} depth={props.depth + 1} />
  );

  const allSwitches = keys.length > 3 && keys.every((k) => {
    const p = resolve(s.properties[k], props.root);
    return typeOf(p) === "boolean" || (Array.isArray(p.type) && p.type.includes("boolean"));
  });
  if (allSwitches) {
    return (
      <div className="switch-grid">
        {keys.map((k) => (
          <label key={k} className="switch" title={hint(resolve(s.properties[k], props.root))}>
            <input type="checkbox" checked={obj[k] === true} onChange={(e) => set(k, e.target.checked ? true : undefined)} />
            <span>{human(k)}</span>
          </label>
        ))}
      </div>
    );
  }

  const main = keys.filter((k) => required.includes(k));
  const more = keys.filter((k) => !required.includes(k));
  const setCount = more.filter((k) => obj[k] !== undefined).length;
  return (
    <div className="fields">
      {main.map(field)}
      {more.length > 0 &&
        (main.length === 0 && props.depth > 0 ? (
          more.map(field)
        ) : (
          <details className="more" open={props.depth > 0 && more.length <= 3}>
            <summary>
              {props.depth === 0 ? t.create.moreSettings : `${t.create.moreSettings}`} <span className="muted">({more.length}{setCount ? `, ${setCount} set` : ""})</span>
            </summary>
            <div className="fields">{more.map(field)}</div>
          </details>
        ))}
    </div>
  );
}

function Label(props: { name: string; schema: Schema; required: boolean; children: React.ReactNode; missing?: boolean }) {
  return (
    <div className="field">
      <span className="field-label">
        {human(props.name)}
        {props.missing ? <span className="needed">{t.create.needed}</span> : null}
      </span>
      {props.children}
      {hint(props.schema) ? <span className="field-hint">{hint(props.schema)}</span> : null}
    </div>
  );
}

function SchemaField(props: FieldProps) {
  const { root, name } = props;
  const s = resolve(props.schema, root);

  // A fixed value — nothing to choose.
  if (constOf(s) !== undefined) return null;

  const base = nullable(s, root);
  if (base) return <Optional {...props} schema={{ ...base, description: s.description ?? base.description }} />;

  const ch = choices(s, root);
  if (ch) {
    const options = ch.map((c) => ({ value: String(c), label: typeof c === "string" ? human(c) : String(c) }));
    return (
      <Label name={name} schema={s} required={props.required}>
        {options.length <= 5 ? (
          <Segmented value={String(props.value ?? ch[0])} options={options} onChange={(v) => props.onChange(ch.find((c) => String(c) === v))} />
        ) : (
          <select value={String(props.value ?? "")} onChange={(e) => props.onChange(ch.find((c) => String(c) === e.target.value))}>
            {options.map((o) => (
              <option key={o.value} value={o.value}>{o.label}</option>
            ))}
          </select>
        )}
      </Label>
    );
  }

  const list = variants(s, root);
  if (list) return <Variants {...props} schema={s} list={list} />;

  switch (typeOf(s)) {
    case "boolean":
      return (
        <label className="switch switch-line">
          <input type="checkbox" checked={props.value === true} onChange={(e) => props.onChange(e.target.checked)} />
          <span>{human(name)}</span>
          {hint(s) ? <span className="field-hint">{hint(s)}</span> : null}
        </label>
      );
    case "integer":
    case "number":
      return (
        <Label name={name} schema={s} required={props.required}>
          <input type="number" min={s.minimum} step={typeOf(s) === "integer" ? 1 : "any"} value={typeof props.value === "number" ? props.value : ""} onChange={(e) => props.onChange(e.target.value === "" ? (props.required ? 0 : undefined) : Number(e.target.value))} />
        </Label>
      );
    case "string": {
      const missing = props.required && !props.value;
      const secret = SECRET.test(name);
      return (
        <Label name={name} schema={s} required={props.required} missing={missing}>
          {LONG.test(name) && !secret ? (
            <textarea rows={name === "python" ? 8 : 3} className={name === "python" ? "mono" : ""} value={String(props.value ?? "")} onChange={(e) => props.onChange(e.target.value)} />
          ) : (
            <input type={secret ? "password" : "text"} value={String(props.value ?? "")} onChange={(e) => props.onChange(e.target.value)} spellCheck={false} />
          )}
        </Label>
      );
    }
    case "array":
      return <ArrayField {...props} schema={s} />;
    case "object":
      if (s.properties) {
        return (
          <fieldset className="group">
            <legend>{human(name)}</legend>
            {hint(s) ? <p className="field-hint">{hint(s)}</p> : null}
            <ObjectFields schema={s} root={root} value={props.value} onChange={props.onChange} depth={props.depth} />
          </fieldset>
        );
      }
      if (s.additionalProperties && typeof s.additionalProperties === "object") return <MapField {...props} schema={s} />;
      return <JsonField {...props} schema={s} />;
    default:
      return <JsonField {...props} schema={s} />;
  }
}

function Optional(props: FieldProps) {
  const { schema: s, root, name } = props;
  const set = props.value !== undefined && props.value !== null;
  const ty = typeOf(s);
  const ch = choices(s, root);
  if (ty === "boolean") {
    const v = props.value === true ? "on" : props.value === false ? "off" : "default";
    return (
      <Label name={name} schema={s} required={false}>
        <Segmented value={v} options={[{ value: "default", label: t.create.default }, { value: "on", label: t.create.on }, { value: "off", label: t.create.off }]} onChange={(x) => props.onChange(x === "default" ? undefined : x === "on")} />
      </Label>
    );
  }
  if (ch) {
    return (
      <Label name={name} schema={s} required={false}>
        <select value={set ? String(props.value) : ""} onChange={(e) => props.onChange(e.target.value === "" ? undefined : ch.find((c) => String(c) === e.target.value))}>
          <option value="">{t.create.default}</option>
          {ch.map((c) => (
            <option key={String(c)} value={String(c)}>{typeof c === "string" ? human(c) : String(c)}</option>
          ))}
        </select>
      </Label>
    );
  }
  if (ty === "string") {
    return (
      <Label name={name} schema={s} required={false}>
        {LONG.test(name) ? (
          <textarea rows={3} value={String(props.value ?? "")} onChange={(e) => props.onChange(e.target.value === "" ? undefined : e.target.value)} />
        ) : (
          <input type={SECRET.test(name) ? "password" : "text"} value={String(props.value ?? "")} placeholder={t.create.default} onChange={(e) => props.onChange(e.target.value === "" ? undefined : e.target.value)} spellCheck={false} />
        )}
      </Label>
    );
  }
  if (ty === "integer" || ty === "number") {
    return (
      <Label name={name} schema={s} required={false}>
        <input type="number" min={s.minimum} step={ty === "integer" ? 1 : "any"} placeholder={t.create.default} value={typeof props.value === "number" ? props.value : ""} onChange={(e) => props.onChange(e.target.value === "" ? undefined : Number(e.target.value))} />
      </Label>
    );
  }
  // Something bigger: switch it on to fill it in.
  return (
    <div className="optional-group">
      <label className="switch switch-line">
        <input type="checkbox" checked={set} onChange={(e) => props.onChange(e.target.checked ? initial(s, root) ?? {} : undefined)} />
        <span>{human(name)}</span>
        {hint(s) ? <span className="field-hint">{hint(s)}</span> : null}
      </label>
      {set ? <div className="optional-body"><SchemaField {...props} schema={s} required /></div> : null}
    </div>
  );
}

function Variants(props: FieldProps & { list: Schema[] }) {
  const { root, list, name } = props;
  const disc = discriminator(list, root);
  const labels = list.map((v, i) => variantLabel(v, root, disc, i));
  const current = (() => {
    if (disc && props.value && typeof props.value === "object") {
      const tag = (props.value as Record<string, Value>)[disc];
      const at = list.findIndex((v) => constOf(resolve(v.properties[disc], root)) === tag);
      if (at >= 0) return at;
    }
    const at = list.findIndex((v) => matches(v, props.value));
    return at >= 0 ? at : 0;
  })();
  const pick = (i: number) => props.onChange(initial(list[i], root));
  const chosen = list[current];
  return (
    <div className="field">
      <span className="field-label">{human(name)}</span>
      {labels.length <= 4 ? (
        <Segmented value={String(current)} options={labels.map((l, i) => ({ value: String(i), label: l }))} onChange={(v) => pick(Number(v))} />
      ) : (
        <select value={current} onChange={(e) => pick(Number(e.target.value))}>
          {labels.map((l, i) => (
            <option key={i} value={i}>{l}</option>
          ))}
        </select>
      )}
      {hint(props.schema) ? <span className="field-hint">{hint(props.schema)}</span> : null}
      <div className="variant-body">
        {typeOf(chosen) === "object" && chosen.properties ? (
          <ObjectFields schema={chosen} root={root} value={props.value} onChange={props.onChange} depth={props.depth} omit={disc ?? undefined} />
        ) : (
          <SchemaField {...props} schema={chosen} required />
        )}
      </div>
    </div>
  );
}

function ArrayField(props: FieldProps) {
  const { schema: s, root, name } = props;
  const items = resolve(s.items ?? {}, root);
  const arr = Array.isArray(props.value) ? (props.value as Value[]) : [];
  if (typeOf(items) === "string" && !choices(items, root)) {
    return (
      <Label name={name} schema={s} required={props.required}>
        <textarea rows={Math.min(6, Math.max(2, arr.length + 1))} value={arr.join("\n")} placeholder="One per line" onChange={(e) => props.onChange(e.target.value.split("\n").filter((line, i, all) => line !== "" || i < all.length - 1))} />
      </Label>
    );
  }
  const set = (i: number, v: Value) => props.onChange(arr.map((x, j) => (j === i ? v : x)));
  return (
    <fieldset className="group">
      <legend>{human(name)}</legend>
      {hint(s) ? <p className="field-hint">{hint(s)}</p> : null}
      {arr.map((item, i) => (
        <div className="array-item" key={i}>
          <SchemaField name={`${name.replace(/s$/, "")} ${i + 1}`} schema={items} root={root} value={item} onChange={(v) => set(i, v)} required depth={props.depth + 1} />
          <button type="button" className="btn btn-quiet btn-small" onClick={() => props.onChange(arr.filter((_, j) => j !== i))}>{t.create.remove}</button>
        </div>
      ))}
      <button type="button" className="btn btn-quiet btn-small" onClick={() => props.onChange([...arr, initial(items, root)])}>+ {t.create.add}</button>
    </fieldset>
  );
}

function MapField(props: FieldProps) {
  const { schema: s, root, name } = props;
  const valueSchema = resolve(s.additionalProperties, root);
  const obj = (props.value && typeof props.value === "object" ? props.value : {}) as Record<string, Value>;
  const [draft, setDraft] = useState("");
  const rename = (from: string, to: string) => {
    const next: Record<string, Value> = {};
    for (const [k, v] of Object.entries(obj)) next[k === from ? to : k] = v;
    props.onChange(next);
  };
  return (
    <fieldset className="group">
      <legend>{human(name)}</legend>
      {hint(s) ? <p className="field-hint">{hint(s)}</p> : null}
      {Object.entries(obj).map(([k, v]) => (
        <div className="map-row" key={k}>
          <input className="map-key" value={k} onChange={(e) => rename(k, e.target.value)} spellCheck={false} />
          <div className="map-value">
            <SchemaField name={t.create.value} schema={valueSchema} root={root} value={v} onChange={(nv) => props.onChange({ ...obj, [k]: nv })} required depth={props.depth + 1} />
          </div>
          <button type="button" className="btn btn-quiet btn-small" onClick={() => { const next = { ...obj }; delete next[k]; props.onChange(next); }}>{t.create.remove}</button>
        </div>
      ))}
      <div className="map-row">
        <input className="map-key" value={draft} placeholder={t.create.key} onChange={(e) => setDraft(e.target.value)} spellCheck={false} />
        <button type="button" className="btn btn-quiet btn-small" disabled={!draft || draft in obj} onClick={() => { props.onChange({ ...obj, [draft]: initial(valueSchema, root) }); setDraft(""); }}>+ {t.create.add}</button>
      </div>
    </fieldset>
  );
}

function JsonField(props: FieldProps) {
  const [text, setText] = useState(() => (props.value === undefined ? "" : JSON.stringify(props.value, null, 2)));
  const [bad, setBad] = useState(false);
  return (
    <Label name={props.name} schema={props.schema} required={props.required}>
      <textarea
        className={`mono${bad ? " invalid" : ""}`}
        rows={3}
        value={text}
        placeholder={t.create.json}
        onChange={(e) => {
          setText(e.target.value);
          if (e.target.value.trim() === "") {
            setBad(false);
            props.onChange(props.required ? {} : undefined);
            return;
          }
          try {
            props.onChange(JSON.parse(e.target.value));
            setBad(false);
          } catch {
            setBad(true);
          }
        }}
      />
    </Label>
  );
}
